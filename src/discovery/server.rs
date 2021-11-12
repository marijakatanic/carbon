use crate::{
    crypto::Identify,
    discovery::{Frame, Request, Response, ServerSettings},
    view::{Install, View},
};

use doomstack::{here, Doom, ResultExt, Top};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use talk::crypto::primitives::hash::Hash;
use talk::net::PlainConnection;
use talk::sync::fuse::Fuse;

use tokio::io;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError as BroadcastRecvError;
use tokio::sync::broadcast::{Receiver as BroadcastReceiver, Sender as BroadcastSender};
use tokio::sync::watch;
use tokio::sync::watch::{Receiver as WatchReceiver, Sender as WatchSender};

use zebra::database::{Collection, CollectionStatus, CollectionTransaction, Family, Question};

type InstallInlet = BroadcastSender<Install>;
type InstallOutlet = BroadcastReceiver<Install>;

type FrameInlet = WatchSender<Arc<Frame>>;
type FrameOutlet = WatchReceiver<Arc<Frame>>;

pub(crate) struct Server {
    address: SocketAddr,
    _fuse: Fuse,
}

struct Database {
    views: HashMap<Hash, View>,
    installs: HashMap<Hash, Install>,
}

struct Sync {
    family: Family<Hash>,
    discovered: Collection<Hash>,
    install_inlet: InstallInlet,
}

struct Publish {
    frame: Arc<Frame>,
    frame_inlet: FrameInlet,
}

#[derive(Doom)]
pub(crate) enum ServerError {
    #[doom(description("Failed to initialize server: {}", source))]
    #[doom(wrap(initialize_failed))]
    InitializeFailed { source: io::Error },
}

#[derive(Doom)]
enum ServeError {
    #[doom(description("Connection error"))]
    ConnectionError,
    #[doom(description("Unexpected request"))]
    UnexpectedRequest,
    #[doom(description("Height overflow"))]
    HeightOverflow,
    #[doom(description("Malformed `Answer`"))]
    MalformedAnswer,
    #[doom(description("Install channel lagged behind"))]
    #[doom(wrap(install_channel_lagged))]
    InstallChannelLagged { source: BroadcastRecvError },
    #[doom(description("`update` failed"))]
    UpdateFailed,
}

#[derive(Doom)]
enum UpdateError {
    #[doom(description("Unknown source view"))]
    UnknownSource,
}

impl Server {
    pub async fn new<A>(
        genesis: View,
        address: A,
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

        let installs = HashMap::new();

        let database = Arc::new(Mutex::new(Database { views, installs }));

        let family = Family::new();
        let discovered = family.empty_collection();

        let (install_inlet, _) = broadcast::channel(settings.update_channel_capacity);

        let sync = Arc::new(Mutex::new(Sync {
            family,
            discovered,
            install_inlet,
        }));

        let frame = Arc::new(Frame::genesis(&genesis));
        let (frame_inlet, frame_outlet) = watch::channel(frame.clone());

        let publish = Arc::new(Mutex::new(Publish { frame, frame_inlet }));

        let fuse = Fuse::new();

        fuse.spawn(async move {
            let _ = Server::listen(listener, database, sync, publish, frame_outlet).await;
        });

        Ok(Server {
            address,
            _fuse: fuse,
        })
    }

    pub(crate) fn address(&self) -> SocketAddr {
        self.address
    }

    async fn listen(
        listener: TcpListener,
        database: Arc<Mutex<Database>>,
        sync: Arc<Mutex<Sync>>,
        publish: Arc<Mutex<Publish>>,
        frame_outlet: FrameOutlet,
    ) {
        let fuse = Fuse::new();

        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let connection: PlainConnection = stream.into();

                let database = database.clone();
                let sync = sync.clone();
                let publish = publish.clone();

                let frame_outlet = frame_outlet.clone();

                fuse.spawn(async move {
                    let _ = Server::serve(connection, database, sync, publish, frame_outlet).await;
                });
            }
        }
    }

    async fn serve(
        mut connection: PlainConnection,
        database: Arc<Mutex<Database>>,
        sync: Arc<Mutex<Sync>>,
        publish: Arc<Mutex<Publish>>,
        frame_outlet: FrameOutlet,
    ) -> Result<(), Top<ServeError>> {
        let request: Request = connection
            .receive()
            .await
            .pot(ServeError::ConnectionError, here!())?;

        match request {
            Request::Publish(install) => {
                Server::serve_publish(connection, database, sync, publish, install).await
            }

            Request::LightSubscribe(height) => {
                // This `Server` cannot handle view height values greater than `usize::MAX`
                if height <= usize::MAX as u64 {
                    Server::serve_light_subscribe(connection, frame_outlet, height as usize).await
                } else {
                    ServeError::HeightOverflow.fail().spot(here!())
                }
            }

            Request::FullSubscribe => {
                Server::serve_full_subscribe(connection, database, sync).await
            }

            _ => ServeError::UnexpectedRequest.fail().spot(here!()),
        }
    }

    async fn serve_publish(
        mut connection: PlainConnection,
        database: Arc<Mutex<Database>>,
        sync: Arc<Mutex<Sync>>,
        publish: Arc<Mutex<Publish>>,
        install: Install,
    ) -> Result<(), Top<ServeError>> {
        Server::update(database, sync, publish, install.clone())
            .pot(ServeError::UpdateFailed, here!())?;

        connection
            .send(&Response::AcknowledgePublish)
            .await
            .pot(ServeError::ConnectionError, here!())?;

        Ok(())
    }

    async fn serve_light_subscribe(
        mut connection: PlainConnection,
        mut frame_outlet: FrameOutlet,
        mut height: usize,
    ) -> Result<(), Top<ServeError>> {
        let mut frame = Some(frame_outlet.borrow_and_update().clone());

        loop {
            if let Some(frame) = frame.take() {
                let installs = frame.lookup(height);
                height = frame.top();

                connection
                    .send(&Response::Update(installs))
                    .await
                    .pot(ServeError::ConnectionError, here!())?;
            }

            tokio::select! {
                biased;

                // `frame_outlet.changed()` returns error only if the `frame_inlet` is closed,
                // in which case the `relay.wait()` will trigger
                Ok(()) = frame_outlet.changed() => {
                    frame = Some(frame_outlet.borrow_and_update().clone());
                }

                request = connection.receive() => {
                    let request = request.pot(ServeError::ConnectionError, here!())?;

                    match request {
                        Request::KeepAlive => {
                            connection.send(&Response::KeepAlive)
                                .await
                                .pot(ServeError::ConnectionError, here!())?;
                        }
                        _ => return ServeError::UnexpectedRequest.fail().spot(here!())
                    }
                }
            }
        }
    }

    async fn serve_full_subscribe(
        mut connection: PlainConnection,
        database: Arc<Mutex<Database>>,
        sync: Arc<Mutex<Sync>>,
    ) -> Result<(), Top<ServeError>> {
        // Remark: the following operations must be executed atomically in order for
        // fully-subscribed clients not to miss (or get redundant) `Install` messages
        // (see `Server::update`)
        let (mut receiver, mut local_discovered, mut install_outlet) = {
            let sync = sync.lock().unwrap();

            (
                sync.family.receive(),
                sync.discovered.clone(),
                sync.install_inlet.subscribe(),
            )
        };

        let mut remote_discovered: Collection<Hash> = loop {
            let answer = connection
                .receive()
                .await
                .pot(ServeError::ConnectionError, here!())?;

            let next = match receiver
                .learn(answer)
                .pot(ServeError::MalformedAnswer, here!())?
            {
                CollectionStatus::Complete(table) => break table,
                CollectionStatus::Incomplete(receiver, question) => (receiver, question),
            };

            receiver = next.0;
            let question = next.1;

            connection
                .send(&Some(question))
                .await
                .pot(ServeError::ConnectionError, here!())?;
        };

        connection
            .send::<Option<Question>>(&None)
            .await
            .pot(ServeError::ConnectionError, here!())?;

        // Compute `local_discovered` set-minus `remote_discovered`

        let gaps = Collection::diff(&mut local_discovered, &mut remote_discovered).0;

        // Query `database` to get appropriate `Install` messages

        let mut updates = {
            let database = database.lock().unwrap();

            gaps.into_iter()
                .map(|diff| {
                    let install = database.installs.get(&diff).unwrap().clone();
                    let height = database.views.get(&install.source()).unwrap().height();

                    (height, install)
                })
                .collect::<Vec<_>>()
        };

        // Sort `updates` by increasing source height

        updates.sort_by_key(|(height, _)| *height);

        let installs = updates
            .into_iter()
            .map(|(_, install)| install)
            .collect::<Vec<_>>();

        // Send all `Install` messages in a `Vec`

        connection
            .send(&Response::Update(installs))
            .await
            .pot(ServeError::ConnectionError, here!())?;

        // Serve updates

        loop {
            tokio::select! {
                biased;

                update = install_outlet.recv() => {
                    let install = update
                        .map_err(ServeError::install_channel_lagged)
                        .map_err(Doom::into_top)
                        .spot(here!())?;

                    connection
                        .send(&Response::Update(vec![install]))
                        .await
                        .pot(ServeError::ConnectionError, here!())?;
                }

                request = connection.receive() => {
                    let request = request.pot(ServeError::ConnectionError, here!())?;

                    match request {
                        Request::KeepAlive => {
                            connection
                                .send(&Response::KeepAlive)
                                .await
                                .pot(ServeError::ConnectionError, here!())?;
                        }
                        _ => return ServeError::UnexpectedRequest.fail().spot(here!()),
                    }
                }
            }
        }
    }

    fn update(
        database: Arc<Mutex<Database>>,
        sync: Arc<Mutex<Sync>>,
        publish: Arc<Mutex<Publish>>,
        install: Install,
    ) -> Result<(), Top<UpdateError>> {
        let identifier = install.identifier();
        let transition = install.clone().into_transition();

        {
            let mut database = database.lock().unwrap();

            // If `transition.source()` is in `database.views`, then
            // `install` is correct and should be acquired (validation
            // of `install` happens on deserialization).
            if !database
                .views
                .contains_key(&transition.source().identifier())
            {
                return UpdateError::UnknownSource.fail();
            }

            // If `install` is in `database.installs` it has already been processed
            // and `Server::update(install, ..)` is a no op.
            if database
                .installs
                .insert(identifier, install.clone())
                .is_some()
            {
                return Ok(());
            }

            // Because `transition.destination()` is reached by `install`,
            // it should be added to the set `database.views` of views
            // that are reachable from `genesis`.
            database.views.insert(
                transition.destination().identifier(),
                transition.destination().clone(),
            );
        }

        // Remark: the following updates must be executed atomically in order for
        // fully-subscribed clients not to miss (or get redundant) `Install` messages
        // (see `serve_light_subscribe`)
        {
            // Add the identifier of `install` to the collection of known valid install messages

            let mut transaction = CollectionTransaction::new();
            transaction.insert(identifier).unwrap();

            let mut sync = sync.lock().unwrap();
            sync.discovered.execute(transaction);

            // Broadcast `install` to the full-subscribe tasks
            let _ = sync.install_inlet.send(install.clone());
        }

        {
            // If `publish.frame` can be updated, broadcast its new value to all
            // light-subscribe tasks
            let mut publish = publish.lock().unwrap();

            if let Some(update) = publish.frame.update(install) {
                publish.frame = Arc::new(update);

                // The corresponding `frame_outlet` is held by `listen`, so this
                // never returns an error until the `Server` is shutting down
                let _ = publish.frame_inlet.send(publish.frame.clone());
            }
        }

        Ok(())
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
        let genesis = generator.view(genesis_height);

        let server = Server::new(genesis, (Ipv4Addr::LOCALHOST, 0), Default::default())
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
            let mut client = Client::new(generator.view(current), generator.view(last_installable));

            let mut client_connection: PlainConnection =
                TcpStream::connect(address).await.unwrap().into();

            client_connection
                .send(&Request::LightSubscribe(client.current().height() as u64))
                .await
                .unwrap();

            let response: Response = client_connection.receive().await.unwrap();

            let installs = match response {
                Response::Update(installs) => installs,
                Response::AcknowledgePublish => panic!("Unexpected second AcknowledgePublish"),
                Response::KeepAlive => panic!("Unexpected KeepAlive when none was sent"),
            };

            client.update(installs.clone());

            assert!(client.current().height() >= expected_server_top);
        }
    }

    #[tokio::test]
    async fn client_continuously_updating() {
        const GENESIS_HEIGHT: usize = 10;
        const MAX_HEIGHT: usize = 100;

        let (server, generator) = setup(GENESIS_HEIGHT, MAX_HEIGHT).await;
        let installs =
            generate_installs(GENESIS_HEIGHT, MAX_HEIGHT, MAX_HEIGHT / 5, MAX_HEIGHT / 15);

        let mut client = Client::new(
            generator.view(GENESIS_HEIGHT),
            generator.view(GENESIS_HEIGHT),
        );

        let mut client_connection: PlainConnection =
            TcpStream::connect(server.address()).await.unwrap().into();

        client_connection
            .send(&Request::LightSubscribe(client.current().height() as u64))
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

            let install = generator.install(source, destination, tail);

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

                client.update(installs);

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
            generate_installs(GENESIS_HEIGHT, MAX_HEIGHT, MAX_HEIGHT / 5, MAX_HEIGHT / 15);

        let mut client_connection: PlainConnection =
            TcpStream::connect(server.address()).await.unwrap().into();

        client_connection
            .send(&Request::LightSubscribe(GENESIS_HEIGHT as u64))
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

            let install = generator.install(source, destination, tail);

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
            generate_installs(GENESIS_HEIGHT, MAX_HEIGHT, MAX_HEIGHT / 5, MAX_HEIGHT / 15);

        let mut client_connection: PlainConnection =
            TcpStream::connect(server.address()).await.unwrap().into();

        client_connection
            .send(&Request::LightSubscribe(GENESIS_HEIGHT as u64))
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

            let install = generator.install(source, destination, tail.clone());

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
