use crate::{
    brokers::signup::{BrokerFailure, BrokerSettings},
    crypto::Identify,
    processing::messages::{SignupRequest, SignupResponse},
    signup::{IdAssignment, IdAssignmentAggregator, IdClaim, IdRequest, SignupSettings},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use futures::stream::{FuturesUnordered, StreamExt};

use log::{info, error};

use rayon::prelude::*;

use std::sync::Arc;

use talk::{
    crypto::{Identity, KeyChain},
    link::context::ConnectDispatcher,
    net::{Connector, SessionConnector},
    sync::fuse::Fuse,
};

use tokio::sync::oneshot::Sender;

type OutcomeInlet = Sender<Result<IdAssignment, BrokerFailure>>;

pub(crate) struct FastSignupBroker {}

#[derive(Debug)]
struct Brokerage {
    request: IdRequest,
    outcome_inlet: OutcomeInlet,
}

#[derive(Debug, Clone)]
struct Collision {
    brokered: IdClaim,
    collided: IdClaim,
}

#[derive(Doom)]
enum SubmitError {
    #[doom(description("Failed to establish a connection"))]
    ConnectionFailed,
    #[doom(description("Connection error"))]
    ConnectionError,
    #[doom(description("Unexpected response"))]
    UnexpectedResponse,
    #[doom(description("Malformed response"))]
    MalformedResponse,
    #[doom(description("Invalid allocation"))]
    InvalidAllocation,
    #[doom(description("Invalid shard"))]
    InvalidShard,
    #[doom(description("Invalid claim"))]
    InvalidClaim,
    #[doom(description("Not a collision"))]
    NotACollision,
    #[doom(description("Insufficient multiplicity to reach a quorum"))]
    MultiplicityInsufficient,
}

impl FastSignupBroker {
    pub async fn signup<C>(view: View, connector: C, settings: BrokerSettings)
    where
        C: Connector,
    {
        let dispatcher = ConnectDispatcher::new(connector);
        let context = format!("{:?}::processor::signup", view.identifier());
        let connector = Arc::new(SessionConnector::new(dispatcher.register(context)));

        let signup_settings = settings.signup_settings;

        info!("Starting signup...");

        let _assignments = FastSignupBroker::flush(10, 5000, view, connector, signup_settings).await;

        info!("Signup complete!");
    }

    async fn flush(
        batches: usize,
        batch_size: usize,
        view: View,
        connector: Arc<SessionConnector>,
        signup_settings: SignupSettings,
    ) -> Vec<(KeyChain, IdAssignment)> {
        let allocator = view.members().iter().next().unwrap().0.clone();

        info!("Generating sigs...");

        let (keychains, requests): (Vec<Vec<KeyChain>>, Vec<Vec<IdRequest>>) = (0..batches)
            .into_par_iter()
            .map(|i| {
                let (batch_key_chains, batch_requests) = (0..batch_size).map(|_| {
                    let keychain = KeyChain::random();
                    let request = IdRequest::new(&keychain, &view, allocator.clone(), 0);

                    (keychain, request)
                }).unzip();

                info!("Generated sigs for batch {}/{}", i + 1, batches);

                (batch_key_chains, batch_requests)
            })
            .unzip();

        let mut joint = keychains
            .into_iter()
            .flatten()
            .zip(requests.into_iter().flatten())
            .collect::<Vec<_>>();

        joint.par_sort_by_key(|(keychain, _)| keychain.keycard().identity());

        let (keychains, requests): (Vec<Vec<KeyChain>>, Vec<Vec<IdRequest>>) = joint
            .chunks(50000)
            .map(|v| Vec::<(KeyChain, IdRequest)>::from(v).into_iter().unzip())
            .unzip();

        info!("Finished generating signatures...");

        let fuse = Fuse::new();

        let mut handles = vec![];
        for request in requests {
            let view = view.clone();
            let connector = connector.clone();
            let signup_settings = signup_settings.clone();

            info!("Signing up batch...");

            let handle = fuse.spawn(async move {
                FastSignupBroker::broker(
                    view,
                    allocator.clone(),
                    connector,
                    request,
                    signup_settings,
                )
                .await
            });

            handles.push(handle);
        }

        info!("Waiting for batch sign up...");

        let mut id_assignments = Vec::new();
        for handle in handles {
            id_assignments.push(handle.await.unwrap().unwrap());
        }

        info!("Internal sign up complete");

        let assignments = keychains
            .into_iter()
            .flatten()
            .zip(id_assignments.into_iter().flatten())
            .collect::<Vec<(KeyChain, IdAssignment)>>();

        for (keychain, assignment) in assignments.iter() {
            assert_eq!(
                assignment.keycard().identity(),
                keychain.keycard().identity()
            );
        }

        assignments
    }

    // Contract: all `brokerages` provided to `Broker::broker` are eventually resolved
    async fn broker(
        view: View,
        allocator: Identity,
        connector: Arc<SessionConnector>,
        requests: Vec<IdRequest>,
        signup_settings: SignupSettings,
    ) -> Vec<IdAssignment> {
        match FastSignupBroker::submit(
            &view,
            allocator,
            connector.as_ref(),
            requests,
            &signup_settings,
        )
        .await
        {
            Ok(assignments) => {
                let assignments: Vec<IdAssignment> =
                    assignments.into_iter().filter_map(Result::ok).collect();
                FastSignupBroker::publish_assignments(
                    &view,
                    connector.as_ref(),
                    assignments.clone(),
                )
                .await;
                return assignments;
            }
            Err(e) => {
                panic!("Assignments not OK: {:?}", e);
            }
        }
    }

    async fn submit(
        view: &View,
        allocator: Identity,
        connector: &SessionConnector,
        requests: Vec<IdRequest>,
        signup_settings: &SignupSettings,
    ) -> Result<Vec<Result<IdAssignment, Collision>>, Top<SubmitError>> {
        let claims = FastSignupBroker::submit_requests(allocator, connector, requests).await?;
        let assignments =
            FastSignupBroker::submit_claims(view, connector, claims, signup_settings).await?;

        Ok(assignments)
    }

    async fn submit_requests(
        allocator: Identity,
        connector: &SessionConnector,
        requests: Vec<IdRequest>,
    ) -> Result<Vec<IdClaim>, Top<SubmitError>> {
        let (requests, allocations) = {
            // Build and submit `SignupRequest::IdRequests` to `allocator`

            let request = SignupRequest::IdRequests(requests);
            let response = FastSignupBroker::request(allocator, connector, &request).await?;
            let requests = request.unwrap_id_requests();

            // Extract unvalidated `allocations` from `allocator`'s `response`

            let allocations = match response {
                SignupResponse::IdAllocations(allocations) => allocations,
                _ => {
                    return SubmitError::UnexpectedResponse.fail().spot(here!());
                }
            };

            (requests, allocations)
        };

        // Validate `allocations`

        // Each element of `allocations` must match  a corresponding element of `requests`
        if allocations.len() != requests.len() {
            return SubmitError::MalformedResponse.fail().spot(here!());
        }

        // Zip `requests` and `allocations` into `claims`
        let claims = requests
            .into_iter()
            .zip(allocations)
            .map(|(request, allocation)| {
                // Each `allocation` must be valid against the corresponding `request`
                allocation
                    .validate(&request)
                    .pot(SubmitError::InvalidAllocation, here!())?;

                Ok(IdClaim::new(request, allocation))
            })
            .collect::<Result<Vec<IdClaim>, Top<SubmitError>>>()?;

        Ok(claims)
    }

    async fn submit_claims(
        view: &View,
        connector: &SessionConnector,
        claims: Vec<IdClaim>,
        signup_settings: &SignupSettings,
    ) -> Result<Vec<Result<IdAssignment, Collision>>, Top<SubmitError>> {
        // Build `SignupRequest::IdClaims`

        let request = SignupRequest::IdClaims(claims.clone());

        // Concurrently submit `request` to all members of `view`

        let mut unordered = view
            .members()
            .iter()
            .map(|(assigner_identity, assigner_keycard)| {
                let assigner_identity = assigner_identity.clone();

                // Futures are processed in `Unordered` fashion. In order to simplify
                // subsequent processing, each future returns, along with its
                // result, the keycard of the relevant assigner
                let assigner_keycard = assigner_keycard.clone();

                let request = &request;

                async move {
                    let result = async {
                        // Submit `request` to `assigner_identity`

                        let response =
                            FastSignupBroker::request(assigner_identity, connector, request)
                                .await?;

                        // Extract unvalidated `shards` from `response`

                        match response {
                            SignupResponse::IdAssignmentShards(shards) => Ok(shards),
                            _ => SubmitError::UnexpectedResponse.fail().spot(here!()),
                        }
                    }
                    .await;

                    (assigner_keycard, result)
                }
            })
            .collect::<FuturesUnordered<_>>();

        // At all times, each element of `slots` contains:
        //  - An `Ok(IdAssignmentAggregator)`, if no collision was found to the
        //    corresponding element of `claims`
        //  - A `Collision` otherwise
        // Upon collecting a quorum of valid assignment shards from the members of `view`,
        // each aggregator in `slots` is `finalize`d into the appropriate `MultiSignature`.
        let mut slots: Vec<Result<IdAssignmentAggregator, Collision>> = claims
            .iter()
            .map(|claim| {
                Ok(IdAssignmentAggregator::new(
                    view.clone(),
                    claim.id(),
                    claim.client(),
                ))
            })
            .collect::<Vec<_>>();

        // At all times, `multiplicity` counts the valid shards received from
        // the members of `view`
        let mut multiplicity = 0;

        while let Some((assigner, result)) = unordered.next().await {
            // Extract unvalidated `shards` from `result`

            let shards = match result {
                Ok(shards) => shards,
                Err(e) => {
                    error!("{:?}", e);
                    continue;
                }
            };

            // Apply `shards` to `slots`

            let result = async {
                // Each element of `shards` must match  a corresponding element of `claims`
                if shards.len() != claims.len() {
                    return SubmitError::MalformedResponse.fail().spot(here!());
                }

                // `progress` zips together corresponding elements of `claims`, `shards`, and
                // `slots`, selecting only those `slots` that still contain an `aggregator`
                let progress = claims
                    .iter()
                    .zip(shards)
                    .zip(slots.iter_mut())
                    .filter(|(_, aggregator)| aggregator.is_ok());

                // Each element of `claims` is denoted `brokered_claim` to distinguish it from
                // a potential `collided_claim` exhibited by `assigner`
                for ((brokered_claim, shard), slot) in progress {
                    match shard {
                        Ok(signature) => {
                            // Try to aggregate `signature` to `slot`'s inner aggregator:
                            // this fails if `signature` is invalid
                            slot.as_mut()
                                .unwrap()
                                .add(&assigner, signature)
                                .pot(SubmitError::InvalidShard, here!())?;
                        }
                        Err(collided_claim) => {
                            // Validate `collided_claim`

                            collided_claim
                                .validate(signup_settings.work_difficulty)
                                .pot(SubmitError::InvalidClaim, here!())?;

                            // `collided_claim` must claim the same id for a different client
                            if collided_claim.id() != brokered_claim.id()
                                || collided_claim.client() == brokered_claim.client()
                            {
                                return SubmitError::NotACollision.fail().spot(here!());
                            }

                            // Place a `Collision` in `slot`

                            let collision = Collision {
                                brokered: brokered_claim.clone(),
                                collided: collided_claim,
                            };

                            *slot = Err(collision);
                        }
                    }
                }

                Ok(())
            }
            .await;

            // `result` is `Ok` only if all `shards` are correctly validated.
            // As a result, because signatures are aggregated on the fly, some
            // aggregators in `slots` might aggregate more than `multiplicity`
            // signatures. This, however, is not a a security issue, and is expected
            // to happen very rarely (i.e., upon accountable replica misbehaviour).
            if result.is_ok() {
                multiplicity += 1;
            } else {
                error!("{:?}", result);
            }

            // At least each aggregator in `slots` has a quorum of signatures: finalize and return
            if multiplicity >= view.quorum() {
                let assignments = slots
                    .into_iter()
                    .map(|slot| {
                        // If `slot` contains an `aggregator`, finalize `aggregator`;
                        // otherwise, preserve the `Collision`
                        slot.map(|aggregator| aggregator.finalize())
                    })
                    .collect::<Vec<_>>();

                return Ok(assignments);
            }
        }

        // Most likely due to network issues, an insufficient number of
        // signatures could be collected from `assignments`.
        // This function can provide proofs of misbehaviour that are,
        // however, not collected at the moment.
        SubmitError::MultiplicityInsufficient.fail().spot(here!())
    }

    async fn publish_assignments(
        view: &View,
        connector: &SessionConnector,
        assignments: Vec<IdAssignment>,
    ) {
        let request = SignupRequest::IdAssignments(assignments);

        let unordered = view
            .members()
            .keys()
            .map(|target| FastSignupBroker::request(*target, connector, &request))
            .collect::<FuturesUnordered<_>>();

        unordered.collect::<Vec<_>>().await;
    }

    async fn request(
        replica: Identity,
        connector: &SessionConnector,
        request: &SignupRequest,
    ) -> Result<SignupResponse, Top<SubmitError>> {
        let mut session = connector
            .connect(replica)
            .await
            .pot(SubmitError::ConnectionFailed, here!())?;

        session
            .send(&request)
            .await
            .pot(SubmitError::ConnectionError, here!())?;

        let response = session
            .receive::<SignupResponse>()
            .await
            .pot(SubmitError::ConnectionError, here!())?;

        session.end();

        Ok(response)
    }
}

impl Into<BrokerFailure> for Collision {
    fn into(self) -> BrokerFailure {
        BrokerFailure::Collision {
            brokered: self.brokered,
            collided: self.collided,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::brokers::test::System;

    use talk::{crypto::KeyChain, net::PlainConnection};

    use tokio::net::TcpStream;

    #[tokio::test]
    async fn stress() {
        let System {
            view,
            discovery_server: _discovery_server,
            discovery_client,
            processors,
            mut signup_brokers,
            ..
        } = System::setup(4, 1, 0).await;

        let discovery_client = Arc::new(discovery_client);

        let signup_broker = signup_brokers.remove(0);
        let allocator_identity = processors[0].0.keycard().identity();

        let client_keychains = (0..16).map(|_| KeyChain::random()).collect::<Vec<_>>();

        let client_keycards = client_keychains
            .iter()
            .map(KeyChain::keycard)
            .collect::<Vec<_>>();

        let requests = client_keychains.iter().map(|client| {
            IdRequest::new(
                client,
                &view,
                allocator_identity,
                SignupSettings::default().work_difficulty,
            )
        });

        let tasks = client_keycards
            .into_iter()
            .zip(requests)
            .map(|(client, request)| {
                let address = signup_broker.address().clone();
                let discovery_client = discovery_client.clone();

                tokio::spawn(async move {
                    let stream = TcpStream::connect(address).await.unwrap();
                    let mut connection: PlainConnection = stream.into();

                    connection.send(&request).await.unwrap();

                    let assignment = connection
                        .receive::<Result<IdAssignment, BrokerFailure>>()
                        .await
                        .unwrap()
                        .unwrap();

                    assignment.validate(discovery_client.as_ref()).unwrap();
                    assert_eq!(*assignment.keycard(), client);
                })
            })
            .collect::<Vec<_>>();

        for task in tasks {
            task.await.unwrap();
        }
    }
}
