use crate::{
    brokers::signup::{BrokerFailure, BrokerSettings},
    crypto::Identify,
    data::Sponge,
    processing::messages::{SignupRequest, SignupResponse},
    signup::{IdAssignment, IdAssignmentAggregator, IdClaim, IdRequest, SignupSettings},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use futures::stream::{FuturesUnordered, StreamExt};

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use talk::{
    crypto::Identity,
    link::context::ConnectDispatcher,
    net::{Connector, PlainConnection, SessionConnector},
    sync::fuse::Fuse,
};

use tokio::{
    io,
    net::{TcpListener, ToSocketAddrs},
    sync::oneshot::{self, Receiver, Sender},
};

type OutcomeInlet = Sender<Result<IdAssignment, BrokerFailure>>;
type OutcomeOutlet = Receiver<Result<IdAssignment, BrokerFailure>>;

pub(crate) struct Broker {
    address: SocketAddr,
    _fuse: Fuse,
}

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
pub(crate) enum BrokerError {
    #[doom(description("Failed to initialize broker: {}", source))]
    #[doom(wrap(initialize_failed))]
    InitializeFailed { source: io::Error },
}

#[derive(Doom)]
enum ServeError {
    #[doom(description("Connection error"))]
    ConnectionError,
    #[doom(description("Request invalid"))]
    RequestInvalid,
    #[doom(description("Request pertains to a foreign view"))]
    ForeignView,
    #[doom(description("Request directed to a foreign allocator"))]
    ForeignAllocator,
    #[doom(description("`Brokerage` forfeited (most likely, the `Broker` is shutting down)"))]
    #[doom(wrap(request_forfeited))]
    BrokerageForfeited { source: oneshot::error::RecvError },
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

impl Broker {
    pub async fn new<A, C>(
        view: View,
        address: A,
        connector: C,
        settings: BrokerSettings,
    ) -> Result<Self, Top<BrokerError>>
    where
        A: ToSocketAddrs,
        C: Connector,
    {
        let listener = TcpListener::bind(address)
            .await
            .map_err(BrokerError::initialize_failed)
            .map_err(Doom::into_top)
            .spot(here!())?;

        let address = listener
            .local_addr()
            .map_err(BrokerError::initialize_failed)
            .map_err(Doom::into_top)
            .spot(here!())?;

        let dispatcher = ConnectDispatcher::new(connector);
        let context = format!("{:?}::processor::signup", view.identifier());
        let connector = Arc::new(SessionConnector::new(dispatcher.register(context)));

        let sponges = Arc::new(
            view.members()
                .keys()
                .map(|member| (*member, Sponge::new(settings.sponge_settings.clone())))
                .collect::<HashMap<_, _>>(),
        );

        let signup_settings = settings.signup_settings;
        let fuse = Fuse::new();

        {
            let view = view.clone();
            let sponges = sponges.clone();
            let signup_settings = signup_settings.clone();

            fuse.spawn(async move {
                Broker::listen(view, sponges, listener, signup_settings).await;
            });
        }

        for allocator in view.members().keys().cloned() {
            let view = view.clone();
            let sponges = sponges.clone();
            let connector = connector.clone();
            let signup_settings = signup_settings.clone();

            fuse.spawn(async move {
                Broker::flush(view, allocator, sponges, connector, signup_settings).await;
            });
        }

        Ok(Broker {
            address,
            _fuse: fuse,
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    async fn listen(
        view: View,
        sponges: Arc<HashMap<Identity, Sponge<Brokerage>>>,
        listener: TcpListener,
        signup_settings: SignupSettings,
    ) {
        let fuse = Fuse::new();

        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let connection: PlainConnection = stream.into();

                let view = view.clone();
                let sponges = sponges.clone();
                let signup_settings = signup_settings.clone();

                fuse.spawn(async move {
                    let _ = Broker::serve(connection, view, sponges, signup_settings).await;
                });
            }
        }
    }

    async fn serve(
        mut connection: PlainConnection,
        view: View,
        sponges: Arc<HashMap<Identity, Sponge<Brokerage>>>,
        signup_settings: SignupSettings,
    ) -> Result<(), Top<ServeError>> {
        let request = connection
            .receive::<IdRequest>()
            .await
            .pot(ServeError::ConnectionError, here!())?;

        request
            .validate(signup_settings.work_difficulty)
            .pot(ServeError::RequestInvalid, here!())?;

        if request.view() != view.identifier() {
            return ServeError::ForeignView.fail().spot(here!());
        }

        let sponge = sponges
            .get(&request.allocator())
            .ok_or(ServeError::ForeignAllocator.into_top().spot(here!()))?;

        let (outcome_inlet, outcome_outlet) = oneshot::channel();

        let brokerage = Brokerage {
            request,
            outcome_inlet,
        };

        sponge.push(brokerage);

        let outcome = outcome_outlet
            .await
            .map_err(ServeError::request_forfeited)
            .map_err(Doom::into_top)
            .spot(here!())?;

        connection
            .send(&outcome)
            .await
            .pot(ServeError::ConnectionError, here!())?;

        Ok(())
    }

    async fn flush(
        view: View,
        allocator: Identity,
        sponges: Arc<HashMap<Identity, Sponge<Brokerage>>>,
        connector: Arc<SessionConnector>,
        signup_settings: SignupSettings,
    ) {
        let sponge = sponges.get(&allocator).unwrap();
        let fuse = Fuse::new();

        loop {
            let brokerages = Broker::prepare(sponge.flush().await);

            if brokerages.is_empty() {
                continue;
            }

            let view = view.clone();
            let connector = connector.clone();
            let signup_settings = signup_settings.clone();

            fuse.spawn(async move {
                Broker::broker(view, allocator, connector, brokerages, signup_settings).await;
            });
        }
    }

    fn prepare(mut brokerages: Vec<Brokerage>) -> Vec<Brokerage> {
        // Sort `brokerages` by requestor

        brokerages.sort_by_key(|brokerage| brokerage.request.client());

        // Deduplicate and fail `brokerages` by requestor

        // The following implementation does not use `Vec::dedup_*` because,
        // in order to fail a duplicate `Brokerage`, it needs to consume
        // its `outcome_inlet` (which mutable references don't allow)
        let mut previous = None;

        brokerages
            .into_iter()
            .filter_map(|brokerage| {
                if Some(brokerage.request.client()) == previous {
                    let _ = brokerage.outcome_inlet.send(Err(BrokerFailure::Throttle));
                    None
                } else {
                    previous = Some(brokerage.request.client());
                    Some(brokerage)
                }
            })
            .collect()
    }

    // Contract: all `brokerages` provided to `Broker::broker` are eventually resolved
    async fn broker(
        view: View,
        allocator: Identity,
        connector: Arc<SessionConnector>,
        brokerages: Vec<Brokerage>,
        signup_settings: SignupSettings,
    ) {
        let (requests, outcome_inlets): (Vec<_>, Vec<_>) = brokerages
            .into_iter()
            .map(|brokerage| (brokerage.request, brokerage.outcome_inlet))
            .unzip();

        match Broker::submit(
            &view,
            allocator,
            connector.as_ref(),
            requests,
            &signup_settings,
        )
        .await
        {
            Ok(assignments) => {
                for (assignment, outcome_inlet) in assignments.iter().cloned().zip(outcome_inlets) {
                    // All `outcome_inlets` are guaranteed to be alive unless `Broker` is shutting down
                    let _ = outcome_inlet.send(assignment.map_err(Into::into));
                }

                let assignments = assignments.into_iter().filter_map(Result::ok).collect();
                Broker::publish_assignments(&view, connector.as_ref(), assignments).await;
            }
            Err(_) => {
                for outcome_inlet in outcome_inlets {
                    // All `outcome_inlets` are guaranteed to be alive unless `Broker` is shutting down
                    let _ = outcome_inlet.send(Err(BrokerFailure::Error));
                }
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
        let claims = Broker::submit_requests(allocator, connector, requests).await?;
        let assignments = Broker::submit_claims(view, connector, claims, signup_settings).await?;

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
            let response = Broker::request(allocator, connector, &request).await?;
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
                            Broker::request(assigner_identity, connector, request).await?;

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
                Err(_) => continue,
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
            .map(|target| Broker::request(*target, connector, &request))
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

    use talk::crypto::KeyChain;

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
                    assert_eq!(assignment.keycard(), client);
                })
            })
            .collect::<Vec<_>>();

        for task in tasks {
            task.await.unwrap();
        }
    }
}
