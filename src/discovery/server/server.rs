use crate::{
    discovery::server::{Frame, Request, Response},
    view::{Install, View},
};

use doomstack::{here, Doom, ResultExt, Top};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use talk::net::PlainConnection;
use talk::sync::fuse::{Fuse, Relay};

use tokio::io;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::broadcast;
use tokio::sync::broadcast::{Receiver as BroadcastReceiver, Sender as BroadcastSender};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{
    Receiver as MpscReceiver, Sender as MpscSender, UnboundedReceiver as UnboundedMpscReceiver,
    UnboundedSender as UnboundedMpscSender,
};

use zebra::Commitment;

type InstallInlet = MpscSender<Install>;
type InstallOutlet = MpscReceiver<Install>;

type UpdateInlet = UnboundedMpscSender<Arc<Frame>>;
type UpdateOutlet = UnboundedMpscReceiver<Arc<Frame>>;

type FrameInlet = BroadcastSender<Arc<Frame>>;
type FrameOutlet = BroadcastReceiver<Arc<Frame>>;

pub(crate) struct Server {
    _fuse: Fuse,
}

struct Database {
    views: HashMap<Commitment, View>,
}

#[derive(Doom)]
pub(crate) enum ServerError {
    #[doom(description("Failed to initialize server: {}", source))]
    #[doom(wrap(initialize_failed))]
    InitializeFailed { source: io::Error },
}

#[derive(Doom)]
enum ListenError {
    #[doom(description("`listen` interrupted"))]
    ListenInterrupted,
}

#[derive(Doom)]
enum ServeError {
    #[doom(description("`serve` interrupted"))]
    ServeInterrupted,
    #[doom(description("connection error"))]
    ConnectionError,
    #[doom(description("Unexpected request"))]
    UnexpectedRequest,
    #[doom(description("Unknown view"))]
    UnknownView,
}

#[derive(Doom)]
enum UpdateError {
    #[doom(description("`update` interrupted"))]
    UpdateInterrupted,
}

impl Server {
    pub async fn new<A>(address: A, genesis: View) -> Result<Self, Top<ServerError>>
    where
        A: ToSocketAddrs,
    {
        let listener = TcpListener::bind(address)
            .await
            .map_err(ServerError::initialize_failed)
            .map_err(Doom::into_top)
            .spot(here!())?;

        let mut views = HashMap::new();
        views.insert(genesis.identifier(), genesis.clone());

        let database = Arc::new(Mutex::new(Database { views }));

        let frame = Arc::new(Frame::genesis(&genesis));

        let (install_inlet, install_outlet) = mpsc::channel(32); // TODO: Add settings
        let (update_inlet, update_outlet) = mpsc::unbounded_channel();

        let fuse = Fuse::new();

        {
            let frame = frame.clone();
            let relay = fuse.relay();

            tokio::spawn(async move {
                let _ = Server::update(frame, install_outlet, update_inlet, relay).await;
            });
        }

        {
            let relay = fuse.relay();

            tokio::spawn(async move {
                let _ = Server::listen(
                    listener,
                    database,
                    frame,
                    install_inlet,
                    update_outlet,
                    relay,
                )
                .await;
            });
        }

        Ok(Server { _fuse: fuse })
    }

    async fn listen(
        listener: TcpListener,
        database: Arc<Mutex<Database>>,
        mut frame: Arc<Frame>,
        install_inlet: InstallInlet,
        mut update_outlet: UpdateOutlet,
        mut relay: Relay,
    ) -> Result<(), Top<ListenError>> {
        let (frame_inlet, _frame_outlet) = broadcast::channel(32); // TODO: Add settings

        let fuse = Fuse::new();

        loop {
            tokio::select! {
                biased;

                _ = relay.wait() => {
                    return ListenError::ListenInterrupted.fail().spot(here!())
                },

                Some(update) = update_outlet.recv() => {
                    frame = update;

                    // This cannot fail, as at least one receiver is permanently
                    // stored in `_frame_outlet`.
                    let _ = frame_inlet.send(frame.clone());
                },

                Ok((stream, _)) = listener.accept() => {
                    let connection: PlainConnection = stream.into();

                    let database = database.clone();
                    let frame = frame.clone();

                    let install_inlet = install_inlet.clone();
                    let frame_outlet = frame_inlet.subscribe();

                    let relay = fuse.relay();

                    tokio::spawn(async move {
                        let _ = Server::serve(database, connection, frame, install_inlet, frame_outlet, relay);
                    });
                }
            }
        }
    }

    async fn serve(
        database: Arc<Mutex<Database>>,
        mut connection: PlainConnection,
        frame: Arc<Frame>,
        install_inlet: InstallInlet,
        frame_outlet: FrameOutlet,
        mut relay: Relay,
    ) -> Result<(), Top<ServeError>> {
        let request: Request = relay
            .map(connection.receive())
            .await
            .pot(ServeError::ServeInterrupted, here!())?
            .pot(ServeError::ConnectionError, here!())?;

        match request {
            Request::Subscribe(height) => {
                Server::serve_subscribe(connection, height as usize, frame, frame_outlet, relay) // TODO: Solve `u64` / `usize` conflict
                    .await
            }
            Request::Publish(install) => {
                Server::serve_publish(database, connection, install, install_inlet, relay).await
            }
            _ => ServeError::UnexpectedRequest.fail().spot(here!()),
        }
    }

    async fn serve_subscribe(
        mut connection: PlainConnection,
        mut height: usize,
        frame: Arc<Frame>,
        mut frame_outlet: FrameOutlet,
        mut relay: Relay,
    ) -> Result<(), Top<ServeError>> {
        let mut frame = Some(frame);

        loop {
            if let Some(frame) = frame.take() {
                let installs = frame.lookup(height);
                height = frame.top();

                relay
                    .map(connection.send(&Response::Update(installs)))
                    .await
                    .pot(ServeError::ServeInterrupted, here!())?
                    .pot(ServeError::ConnectionError, here!())?;
            }

            tokio::select! {
                biased;

                _ = relay.wait() => {
                    return ServeError::ServeInterrupted.fail().spot(here!())
                },

                Ok(update) = frame_outlet.recv() => {
                    frame = Some(update)
                }

                request = connection.receive() => {
                    let request = request.pot(ServeError::ConnectionError, here!())?;

                    match request {
                        Request::KeepAlive => {
                            relay.map(connection.send(&Response::KeepAlive)).await.pot(ServeError::ServeInterrupted, here!())?.pot(ServeError::ConnectionError, here!())?;
                        }
                        _ => return ServeError::UnexpectedRequest.fail().spot(here!())
                    }
                }
            }
        }
    }

    async fn serve_publish(
        database: Arc<Mutex<Database>>,
        mut connection: PlainConnection,
        install: Install,
        install_inlet: InstallInlet,
        mut relay: Relay,
    ) -> Result<(), Top<ServeError>> {
        let transition = install.clone().into_transition().await;
        let mut database = database.lock().unwrap();

        if database
            .views
            .contains_key(&transition.source().identifier())
        {
            database.views.insert(
                transition.destination().identifier(),
                transition.destination().clone(),
            );

            drop(database);

            // Because `install_inlet` is unbounded, this can only fail if the
            // corresponding `install_outlet` is dropped, in which case the
            // `Server` is shutting down and we don't care about the error
            let _ = install_inlet.send(install);

            relay
                .map(connection.send(&Response::AcknowledgePublish))
                .await
                .pot(ServeError::ServeInterrupted, here!())?
                .pot(ServeError::ConnectionError, here!())?;

            Ok(())
        } else {
            ServeError::UnknownView.fail().spot(here!())
        }
    }

    async fn update(
        mut frame: Arc<Frame>,
        mut install_outlet: InstallOutlet,
        update_inlet: UpdateInlet,
        mut relay: Relay,
    ) -> Result<(), Top<UpdateError>> {
        loop {
            if let Some(install) = relay
                .map(install_outlet.recv())
                .await
                .pot(UpdateError::UpdateInterrupted, here!())?
            {
                if let Some(update) = frame.update(install).await {
                    frame = Arc::new(update);
                    let _ = update_inlet.send(frame.clone());
                }
            }
        }
    }
}
