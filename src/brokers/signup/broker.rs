use crate::{
    brokers::signup::Failure,
    crypto::Identify,
    data::Sponge,
    processing::messages::{SignupRequest, SignupResponse},
    signup::{
        IdAllocation, IdAssignment, IdAssignmentAggregator, IdClaim, IdRequest, SignupSettings,
    },
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use futures::stream::{FuturesUnordered, StreamExt};

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use talk::{
    crypto::{primitives::multi::Signature as MultiSignature, Identity},
    link::context::ConnectDispatcher,
    net::{Connector, PlainConnection, SessionConnector},
    sync::fuse::Fuse,
};

use tokio::{
    io,
    net::{TcpListener, ToSocketAddrs},
    sync::oneshot::{self, Receiver, Sender},
};

type OutcomeInlet = Sender<Result<IdAssignment, Failure>>;
type OutcomeOutlet = Receiver<Result<IdAssignment, Failure>>;

pub(crate) struct Broker {
    address: SocketAddr,
    _fuse: Fuse,
}

#[derive(Debug)]
struct Request {
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
    #[doom(description("Request forfeited (most likely, the `Broker` is shutting down)"))]
    #[doom(wrap(request_forfeited))]
    RequestForfeited { source: oneshot::error::RecvError },
}

#[derive(Doom)]
enum DriveError {
    #[doom(description("Failed to establish a connection to the allocator"))]
    AllocatorConnectionFailed,
    #[doom(description("Allocator connection error"))]
    AllocatorConnectionError,
    #[doom(description("Unexpected response from allocator"))]
    UnexpectedAllocatorResponse,
    #[doom(description("Malformed response from allocator"))]
    MalformedAllocatorResponse,
    #[doom(description("Invalid allocation"))]
    InvalidAllocation,
    #[doom(description("Failed to establish a connection to the assigner"))]
    AssignerConnectionFailed,
    #[doom(description("Assigner connection error"))]
    AssignerConnectionError,
    #[doom(description("Unexpected response from assigner"))]
    UnexpectedAssignerResponse,
    #[doom(description("Malformed response from assigner"))]
    MalformedAssignerResponse,
    #[doom(description("Invalid assignment"))]
    InvalidAssignment,
    #[doom(description("Multiplicity insufficient"))]
    MultiplicityInsufficient,
}

impl Broker {
    pub async fn new<A, C>(view: View, address: A, connector: C) -> Result<Self, Top<BrokerError>>
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
                .map(|member| (*member, Sponge::new(Default::default()))) // TODO: Add settings
                .collect::<HashMap<_, _>>(),
        );

        let fuse = Fuse::new();

        {
            let view = view.clone();
            let sponges = sponges.clone();

            fuse.spawn(async move {
                Broker::listen(listener, view, sponges).await;
            });
        }

        for allocator in view.members().keys().cloned() {
            let view = view.clone();
            let sponges = sponges.clone();
            let connector = connector.clone();

            fuse.spawn(async move {
                Broker::flush(view, sponges, allocator, connector).await;
            });
        }

        Ok(Broker {
            address,
            _fuse: fuse,
        })
    }

    async fn listen(
        listener: TcpListener,
        view: View,
        sponges: Arc<HashMap<Identity, Sponge<Request>>>,
    ) {
        let fuse = Fuse::new();

        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let connection: PlainConnection = stream.into();

                let view = view.clone();
                let sponges = sponges.clone();

                fuse.spawn(async move {
                    let _ = Broker::serve(connection, view, sponges).await;
                });
            }
        }
    }

    async fn serve(
        mut connection: PlainConnection,
        view: View,
        sponges: Arc<HashMap<Identity, Sponge<Request>>>,
    ) -> Result<(), Top<ServeError>> {
        let request = connection
            .receive::<IdRequest>()
            .await
            .pot(ServeError::ConnectionError, here!())?;

        request
            .validate(SignupSettings::default().work_difficulty) // TODO: Add settings
            .pot(ServeError::RequestInvalid, here!())?;

        if request.view() != view.identifier() {
            return ServeError::ForeignView.fail().spot(here!());
        }

        let sponge = sponges
            .get(&request.allocator())
            .ok_or(ServeError::ForeignAllocator.into_top().spot(here!()))?;

        let (outcome_inlet, outcome_outlet) = oneshot::channel();

        let request = Request {
            request,
            outcome_inlet,
        };

        sponge.push(request);

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
        sponges: Arc<HashMap<Identity, Sponge<Request>>>,
        allocator: Identity,
        connector: Arc<SessionConnector>,
    ) {
        let sponge = sponges.get(&allocator).unwrap();
        let fuse = Fuse::new();

        loop {
            let requests = sponge.flush().await;
            let view = view.clone();
            let connector = connector.clone();

            fuse.spawn(async move {
                Broker::broker(view, allocator, connector, requests).await;
            });
        }
    }

    async fn broker(
        view: View,
        allocator: Identity,
        connector: Arc<SessionConnector>,
        requests: Vec<Request>,
    ) {
        let (requests, outcome_inlets): (Vec<_>, Vec<_>) = requests
            .into_iter()
            .map(|request| (request.request, request.outcome_inlet))
            .unzip();

        match Broker::drive(view, allocator, connector, requests).await {
            Ok(assignments) => {
                for (assignment, outcome_inlet) in assignments.into_iter().zip(outcome_inlets) {
                    let _ = outcome_inlet.send(assignment.map_err(Into::into));
                }
            }
            Err(_) => {
                for outcome_inlet in outcome_inlets {
                    let _ = outcome_inlet.send(Err(Failure::Network));
                }
            }
        }
    }

    async fn drive(
        view: View,
        allocator: Identity,
        connector: Arc<SessionConnector>,
        requests: Vec<IdRequest>,
    ) -> Result<Vec<Result<IdAssignment, Collision>>, Top<DriveError>> {
        let allocations =
            Broker::submit_id_requests(allocator, connector.as_ref(), requests.clone()).await?;

        if allocations.len() != requests.len() {
            return DriveError::MalformedAllocatorResponse.fail().spot(here!());
        }

        let claims = requests
            .into_iter()
            .zip(allocations)
            .map(|(request, allocation)| {
                allocation
                    .validate(&request)
                    .pot(DriveError::InvalidAllocation, here!())?;
                Ok(IdClaim::new(request, allocation))
            })
            .collect::<Result<Vec<IdClaim>, Top<DriveError>>>()?;

        let mut unordered = view
            .members()
            .iter()
            .map(|(assigner_identity, assigner_keycard)| {
                let assigner_identity = assigner_identity.clone();
                let assigner_keycard = assigner_keycard.clone();

                let claims = claims.clone();
                let connector = connector.as_ref();

                async move {
                    (
                        assigner_keycard,
                        Broker::submit_id_claims(assigner_identity, connector, claims).await,
                    )
                }
            })
            .collect::<FuturesUnordered<_>>();

        let mut aggregators: Vec<Result<IdAssignmentAggregator, Collision>> = claims
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
                return DriveError::MalformedAssignerResponse.fail().spot(here!());
            }

            let progress = aggregators
                .iter_mut()
                .zip(claims.iter())
                .zip(assignments)
                .filter(|((aggregator, _), _)| aggregator.is_ok());

            let result = async {
                for ((aggregator, brokered_claim), assignment) in progress {
                    match assignment {
                        Ok(signature) => {
                            aggregator
                                .as_mut()
                                .unwrap()
                                .add(&assigner, signature)
                                .pot(DriveError::MalformedAssignerResponse, here!())?;
                        }
                        Err(collided_claim) => {
                            let id = aggregator.as_ref().unwrap().id();
                            let client = aggregator.as_ref().unwrap().keycard();

                            collided_claim
                                .validate(SignupSettings::default().work_difficulty)
                                .pot(DriveError::MalformedAssignerResponse, here!())?;

                            if collided_claim.id() != id || collided_claim.client() == client {
                                return DriveError::MalformedAssignerResponse.fail().spot(here!());
                            }

                            let collision = Collision {
                                brokered: brokered_claim.clone(),
                                collided: collided_claim,
                            };

                            *aggregator = Err(collision);
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
                let assignments = aggregators
                    .into_iter()
                    .map(|aggregator| aggregator.map(|aggregator| aggregator.finalize()))
                    .collect::<Vec<_>>();

                return Ok(assignments);
            }
        }

        DriveError::MultiplicityInsufficient.fail().spot(here!())
    }

    async fn submit_id_requests(
        allocator: Identity,
        connector: &SessionConnector,
        requests: Vec<IdRequest>,
    ) -> Result<Vec<IdAllocation>, Top<DriveError>> {
        let mut session = connector
            .connect(allocator)
            .await
            .pot(DriveError::AllocatorConnectionFailed, here!())?;

        session
            .send(&SignupRequest::IdRequests(requests))
            .await
            .pot(DriveError::AllocatorConnectionError, here!())?;

        let response = session
            .receive::<SignupResponse>()
            .await
            .pot(DriveError::AllocatorConnectionError, here!())?;

        session.end();

        let allocations = match response {
            SignupResponse::IdAllocations(allocations) => allocations,
            _ => {
                return DriveError::UnexpectedAllocatorResponse.fail().spot(here!());
            }
        };

        Ok(allocations)
    }

    async fn submit_id_claims(
        assigner: Identity,
        connector: &SessionConnector,
        claims: Vec<IdClaim>,
    ) -> Result<Vec<Result<MultiSignature, IdClaim>>, Top<DriveError>> {
        let mut session = connector
            .connect(assigner)
            .await
            .pot(DriveError::AssignerConnectionFailed, here!())?;

        session
            .send(&SignupRequest::IdClaims(claims))
            .await
            .pot(DriveError::AssignerConnectionError, here!())?;

        let response = session
            .receive::<SignupResponse>()
            .await
            .pot(DriveError::AllocatorConnectionError, here!())?;

        session.end();

        let assignments = match response {
            SignupResponse::IdAssignments(assignments) => assignments,
            _ => {
                return DriveError::UnexpectedAssignerResponse.fail().spot(here!());
            }
        };

        Ok(assignments)
    }
}

impl Into<Failure> for Collision {
    fn into(self) -> Failure {
        Failure::Collision {
            brokered: self.brokered,
            collided: self.collided,
        }
    }
}
