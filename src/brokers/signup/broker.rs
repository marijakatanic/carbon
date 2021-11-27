use crate::{
    crypto::Identify,
    data::Sponge,
    signup::{IdAssignment, IdClaim, IdRequest, SignupSettings},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use talk::{crypto::Identity, net::PlainConnection, sync::fuse::Fuse};

use tokio::{
    io,
    net::{TcpListener, ToSocketAddrs},
    sync::oneshot::{self, Receiver, Sender},
};

type OutcomeInlet = Sender<Result<IdAssignment, Collision>>;
type OutcomeOutlet = Receiver<Result<IdAssignment, Collision>>;

pub(crate) struct Broker {
    address: SocketAddr,
    _fuse: Fuse,
}

struct Request {
    request: IdRequest,
    outcome_inlet: OutcomeInlet,
}

#[derive(Serialize, Deserialize)]
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

impl Broker {
    pub async fn new<A>(view: View, address: A) -> Result<Self, Top<BrokerError>>
    where
        A: ToSocketAddrs,
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

            fuse.spawn(async move {
                Broker::flush(view, sponges, allocator).await;
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
    ) {
        let fuse = Fuse::new();

        loop {
            let requests = sponges[&allocator].flush().await;
            let view = view.clone();

            fuse.spawn(async move {
                Broker::broker(view, allocator, requests).await;
            });
        }
    }

    async fn broker(view: View, allocator: Identity, requests: Vec<Request>) {}
}
