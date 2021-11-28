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

#[derive(Debug)]
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
    #[doom(description("Invalid assignment"))]
    InvalidAssignment,
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
            let brokerages = sponge.flush().await;
            let view = view.clone();
            let connector = connector.clone();
            let signup_settings = signup_settings.clone();

            fuse.spawn(async move {
                Broker::broker(view, allocator, connector, brokerages, signup_settings).await;
            });
        }
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
            view,
            allocator,
            connector.as_ref(),
            requests,
            &signup_settings,
        )
        .await
        {
            Ok(assignments) => {
                for (assignment, outcome_inlet) in assignments.into_iter().zip(outcome_inlets) {
                    // All `outcome_inlets` are guaranteed to be alive unless `Broker` is shutting down
                    let _ = outcome_inlet.send(assignment.map_err(Into::into));
                }
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
        view: View,
        allocator: Identity,
        connector: &SessionConnector,
        requests: Vec<IdRequest>,
        signup_settings: &SignupSettings,
    ) -> Result<Vec<Result<IdAssignment, Collision>>, Top<SubmitError>> {
        let (requests, allocations) = {
            let request = SignupRequest::IdRequests(requests);
            let response = Broker::request(allocator, connector, &request).await?;
            let requests = request.unwrap_id_requests();

            let allocations = match response {
                SignupResponse::IdAllocations(allocations) => allocations,
                _ => {
                    return SubmitError::UnexpectedResponse.fail().spot(here!());
                }
            };

            (requests, allocations)
        };

        if allocations.len() != requests.len() {
            return SubmitError::MalformedResponse.fail().spot(here!());
        }

        let claims = requests
            .into_iter()
            .zip(allocations)
            .map(|(request, allocation)| {
                allocation
                    .validate(&request)
                    .pot(SubmitError::InvalidAllocation, here!())?;

                Ok(IdClaim::new(request, allocation))
            })
            .collect::<Result<Vec<IdClaim>, Top<SubmitError>>>()?;

        let request = SignupRequest::IdClaims(claims.clone());

        let mut unordered = view
            .members()
            .iter()
            .map(|(assigner_identity, assigner_keycard)| {
                let assigner_identity = assigner_identity.clone();
                let assigner_keycard = assigner_keycard.clone();

                let request = &request;

                async move {
                    let result = async {
                        let response =
                            Broker::request(assigner_identity, connector, request).await?;

                        match response {
                            SignupResponse::IdAssignments(assignments) => Ok(assignments),
                            _ => SubmitError::UnexpectedResponse.fail().spot(here!()),
                        }
                    }
                    .await;

                    (assigner_keycard, result)
                }
            })
            .collect::<FuturesUnordered<_>>();

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

        let mut multiplicity = 0;

        while let Some((assigner, result)) = unordered.next().await {
            let assignments = match result {
                Ok(assignments) => assignments,
                Err(_) => continue,
            };

            if assignments.len() != claims.len() {
                return SubmitError::MalformedResponse.fail().spot(here!());
            }

            let progress = claims
                .iter()
                .zip(assignments)
                .zip(slots.iter_mut())
                .filter(|(_, aggregator)| aggregator.is_ok());

            let result = async {
                for ((brokered_claim, assignment), slot) in progress {
                    match assignment {
                        Ok(signature) => {
                            slot.as_mut()
                                .unwrap()
                                .add(&assigner, signature)
                                .pot(SubmitError::InvalidAssignment, here!())?;
                        }
                        Err(collided_claim) => {
                            collided_claim
                                .validate(signup_settings.work_difficulty)
                                .pot(SubmitError::InvalidClaim, here!())?;

                            if collided_claim.id() != brokered_claim.id()
                                || collided_claim.client() == brokered_claim.client()
                            {
                                return SubmitError::NotACollision.fail().spot(here!());
                            }

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

            if result.is_ok() {
                multiplicity += 1;
            }

            if multiplicity >= view.quorum() {
                let assignments = slots
                    .into_iter()
                    .map(|slot| slot.map(|aggregator| aggregator.finalize()))
                    .collect::<Vec<_>>();

                return Ok(assignments);
            }
        }

        SubmitError::MultiplicityInsufficient.fail().spot(here!())
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
