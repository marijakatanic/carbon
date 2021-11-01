use crate::{
    discovery::{Frame, Request, Response, ServerSettings},
    view::{Install, View},
};

use doomstack::{here, Doom, ResultExt, Top};

use std::collections::HashMap;
use std::net::SocketAddr;
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
    address: SocketAddr,
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
    pub async fn new<A>(
        address: A,
        genesis: View,
        settings: ServerSettings,
    ) -> Result<Self, Top<ServerError>>
    where
        A: ToSocketAddrs,
    {
        let listener = TcpListener::bind(address)
            .await
            .map_err(ServerError::initialize_failed)
            .map_err(Doom::into_top)
            .spot(here!())?;

        let address = listener
            .local_addr()
            .map_err(ServerError::initialize_failed)
            .map_err(Doom::into_top)
            .spot(here!())?;

        let mut views = HashMap::new();
        views.insert(genesis.identifier(), genesis.clone());

        let database = Arc::new(Mutex::new(Database { views }));

        let frame = Arc::new(Frame::genesis(&genesis));

        let (install_inlet, install_outlet) = mpsc::channel(settings.install_channel_capacity);
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
                    settings,
                    relay,
                )
                .await;
            });
        }

        Ok(Server {
            _fuse: fuse,
            address: address,
        })
    }

    pub(crate) fn address(&self) -> SocketAddr {
        self.address
    }

    async fn listen(
        listener: TcpListener,
        database: Arc<Mutex<Database>>,
        mut frame: Arc<Frame>,
        install_inlet: InstallInlet,
        mut update_outlet: UpdateOutlet,
        settings: ServerSettings,
        mut relay: Relay,
    ) -> Result<(), Top<ListenError>> {
        // Q: Why keep the _frame_outlet at all?
        let (frame_inlet, _frame_outlet) = broadcast::channel(settings.frame_channel_capacity);

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
                        let _ = Server::serve(database, connection, frame, install_inlet, frame_outlet, relay).await;
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
                // This server cannot handle view height values greater than usize::MAX"
                assert!(height <= usize::MAX as u64);

                Server::serve_subscribe(connection, height as usize, frame, frame_outlet, relay)
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
                            relay.map(connection.send(&Response::KeepAlive))
                                .await
                                .pot(ServeError::ServeInterrupted, here!())?
                                .pot(ServeError::ConnectionError, here!())?;
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

        {
            let mut database = database.lock().unwrap();

            if database
                .views
                .contains_key(&transition.source().identifier())
            {
                database.views.insert(
                    transition.destination().identifier(),
                    transition.destination().clone(),
                );
            } else {
                ServeError::UnknownView.fail().spot(here!())?
            }
        }

        // Because `install_inlet` is unbounded, this can only fail if the
        // corresponding `install_outlet` is dropped, in which case the
        // `Server` is shutting down and we don't care about the error
        let _ = install_inlet.send(install).await;

        relay
            .map(connection.send(&Response::AcknowledgePublish))
            .await
            .pot(ServeError::ServeInterrupted, here!())?
            .pot(ServeError::ConnectionError, here!())?;

        Ok(())
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

#[cfg(test)]
mod test {
    use super::*;

    use crate::view::test::{generate_installs, last_installable, Client, InstallGenerator};

    use std::net::Ipv4Addr;

    use talk::net::PlainConnection;

    use tokio::net::TcpStream;

    async fn setup(genesis_height: usize, max_height: usize) -> (Server, InstallGenerator) {
        let generator = InstallGenerator::new(max_height);
        let genesis = generator.view(genesis_height).await;

        let server = Server::new((Ipv4Addr::UNSPECIFIED, 0), genesis, Default::default())
            .await
            .unwrap();

        (server, generator)
    }

    async fn check_server<I>(
        address: SocketAddr,
        genesis_height: usize,
        expected_server_top: usize,
        tailless: I,
        generator: &InstallGenerator,
    ) where
        I: IntoIterator<Item = usize>,
    {
        for (current, last_installable) in
            last_installable(genesis_height, generator.max_height(), tailless)
                .into_iter()
                .enumerate()
                .filter(|(i, _)| *i >= genesis_height)
        {
            let mut client = Client::new(
                generator.view(current).await,
                generator.view(last_installable).await,
            );

            let mut client_connection: PlainConnection =
                TcpStream::connect(address).await.unwrap().into();

            client_connection
                .send(&Request::Subscribe(client.current().height() as u64))
                .await
                .unwrap();

            let response: Response = client_connection.receive().await.unwrap();

            let installs = match response {
                Response::Update(installs) => installs,
                Response::AcknowledgePublish => panic!("Unexpected second AcknowledgePublish"),
                Response::KeepAlive => panic!("Unexpected KeepAlive when none was sent"),
            };

            client.update(installs.clone()).await;

            assert!(client.current().height() >= expected_server_top);
        }
    }

    #[tokio::test]
    async fn client_continuously_updating() {
        const GENESIS_HEIGHT: usize = 10;
        const MAX_HEIGHT: usize = 100;

        let (server, generator) = setup(GENESIS_HEIGHT, MAX_HEIGHT).await;
        let installs =
            generate_installs(GENESIS_HEIGHT, MAX_HEIGHT, MAX_HEIGHT / 5, MAX_HEIGHT / 15).await;

        let mut client = Client::new(
            generator.view(GENESIS_HEIGHT).await,
            generator.view(GENESIS_HEIGHT).await,
        );

        let mut client_connection: PlainConnection =
            TcpStream::connect(server.address()).await.unwrap().into();

        client_connection
            .send(&Request::Subscribe(client.current().height() as u64))
            .await
            .unwrap();

        match client_connection.receive().await.unwrap() {
            Response::Update(installs) => assert_eq!(installs.len(), 0),
            Response::AcknowledgePublish => panic!("Unexpected second AcknowledgePublish"),
            Response::KeepAlive => panic!("Unexpected KeepAlive when none was sent"),
        };

        let mut tailless = Vec::new();
        let mut expected_top = GENESIS_HEIGHT;

        for (source, destination, tail) in installs {
            if tail.len() == 0 {
                tailless.push(destination);
            }

            let install = generator.install(source, destination, tail).await;

            let mut replica_connection: PlainConnection =
                TcpStream::connect(server.address()).await.unwrap().into();

            replica_connection
                .send(&Request::Publish(install))
                .await
                .unwrap();

            match replica_connection.receive().await.unwrap() {
                Response::AcknowledgePublish => (),
                _ => panic!("Unexpected response"),
            }

            drop(replica_connection);

            if destination > expected_top {
                expected_top = destination;

                let installs = match client_connection.receive().await.unwrap() {
                    Response::Update(installs) => installs,
                    Response::AcknowledgePublish => panic!("Unexpected second AcknowledgePublish"),
                    Response::KeepAlive => panic!("Unexpected KeepAlive when none was sent"),
                };

                client.update(installs).await;

                assert_eq!(client.current().height(), expected_top);
            }
        }

        assert_eq!(client.current().height(), MAX_HEIGHT - 1);
    }

    #[tokio::test]
    async fn client_update_stress_light_checks() {
        const GENESIS_HEIGHT: usize = 10;
        const MAX_HEIGHT: usize = 30;

        let (server, generator) = setup(GENESIS_HEIGHT, MAX_HEIGHT).await;

        let installs =
            generate_installs(GENESIS_HEIGHT, MAX_HEIGHT, MAX_HEIGHT / 5, MAX_HEIGHT / 15).await;

        let mut client_connection: PlainConnection =
            TcpStream::connect(server.address()).await.unwrap().into();

        client_connection
            .send(&Request::Subscribe(GENESIS_HEIGHT as u64))
            .await
            .unwrap();

        let _response: Response = client_connection.receive().await.unwrap();

        let mut tailless = Vec::new();
        let mut expected_top = GENESIS_HEIGHT;

        for (source, destination, tail) in installs {
            let mut replica_connection: PlainConnection =
                TcpStream::connect(server.address()).await.unwrap().into();

            if tail.len() == 0 {
                tailless.push(destination);
            }

            let install = generator.install(source, destination, tail).await;

            replica_connection
                .send(&Request::Publish(install))
                .await
                .unwrap();

            match replica_connection.receive().await.unwrap() {
                Response::AcknowledgePublish => (),
                _ => panic!("Unexpected response"),
            }

            drop(replica_connection);

            if destination > expected_top {
                expected_top = destination;
                let _response: Response = client_connection.receive().await.unwrap();
            }
        }

        check_server(
            server.address(),
            GENESIS_HEIGHT,
            MAX_HEIGHT - 1,
            tailless,
            &generator,
        )
        .await;
    }

    #[tokio::test]
    async fn client_update_stress_heavy_checks() {
        const GENESIS_HEIGHT: usize = 10;
        const MAX_HEIGHT: usize = 30;

        let (server, generator) = setup(GENESIS_HEIGHT, MAX_HEIGHT).await;

        let installs =
            generate_installs(GENESIS_HEIGHT, MAX_HEIGHT, MAX_HEIGHT / 5, MAX_HEIGHT / 15).await;

        let mut client_connection: PlainConnection =
            TcpStream::connect(server.address()).await.unwrap().into();

        client_connection
            .send(&Request::Subscribe(GENESIS_HEIGHT as u64))
            .await
            .unwrap();

        let _response: Response = client_connection.receive().await.unwrap();

        let mut tailless = Vec::new();
        let mut expected_top = GENESIS_HEIGHT;

        for (source, destination, tail) in installs {
            let mut replica_connection: PlainConnection =
                TcpStream::connect(server.address()).await.unwrap().into();

            if tail.len() == 0 {
                tailless.push(destination);
            }

            let install = generator.install(source, destination, tail.clone()).await;

            replica_connection
                .send(&Request::Publish(install))
                .await
                .unwrap();

            match replica_connection.receive().await.unwrap() {
                Response::AcknowledgePublish => (),
                _ => panic!("Unexpected response"),
            }

            drop(replica_connection);

            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

            if destination > expected_top {
                expected_top = destination;
                let _response: Response = client_connection.receive().await.unwrap();
            }

            check_server(
                server.address(),
                GENESIS_HEIGHT,
                expected_top,
                tailless.clone(),
                &generator,
            )
            .await;
        }
    }
}
