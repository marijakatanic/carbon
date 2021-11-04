use crate::{
    discovery::{Frame, Request, Response, ServerSettings},
    view::{Install, View},
};

use doomstack::{here, Doom, ResultExt, Top};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex as StdMutex};

use talk::crypto::primitives::hash;
use talk::crypto::primitives::hash::Hash;
use talk::net::PlainConnection;
use talk::sync::fuse::Fuse;

use tokio::io;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::broadcast;
use tokio::sync::broadcast::{
    error::RecvError as BroadcastRecvError, Receiver as BroadcastReceiver,
    Sender as BroadcastSender,
};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver as MpscReceiver, Sender as MpscSender};
use tokio::sync::watch;
use tokio::sync::watch::{Receiver as WatchReceiver, Sender as WatchSender};
use tokio::sync::Mutex as TokioMutex;

use zebra::database::{Collection, CollectionStatus, CollectionTransaction, Family, Question};
use zebra::Commitment;

type PublicationInlet = MpscSender<Install>;
type PublicationOutlet = MpscReceiver<Install>;

type UpdateInlet = BroadcastSender<Install>;
type UpdateOutlet = BroadcastReceiver<Install>;

type FrameInlet = WatchSender<Arc<Frame>>;
type FrameOutlet = WatchReceiver<Arc<Frame>>;

pub(crate) struct Server {
    _fuse: Fuse,
    address: SocketAddr,
}

struct Database {
    views: HashMap<Commitment, View>,
    installs: HashMap<Hash, Install>,
}

struct Sync {
    family: Family<Hash>,
    discovered: Collection<Hash>,
    update_inlet: UpdateInlet,
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
    #[doom(description("Malformed `Answer`"))]
    MalformedAnswer,
    #[doom(description("Unknown view"))]
    UnknownView,
    #[doom(description("Height overflow"))]
    HeightOverflow,
    #[doom(description("Update channel error (shutdown or lagged behind)"))]
    #[doom(wrap(update_error))]
    UpdateError { source: BroadcastRecvError },
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

        let database = Arc::new(StdMutex::new(Database { views, installs }));

        let family = Family::new();
        let discovered = family.empty_collection();

        let (update_inlet, _) = broadcast::channel(settings.update_channel_capacity);

        let sync = Arc::new(TokioMutex::new(Sync {
            family,
            discovered,
            update_inlet,
        }));

        let frame = Arc::new(Frame::genesis(&genesis));

        let (publication_inlet, publication_outlet) =
            mpsc::channel(settings.install_channel_capacity);

        let (frame_inlet, frame_outlet) = watch::channel(frame.clone());

        let fuse = Fuse::new();

        {
            let sync = sync.clone();

            fuse.spawn(async move {
                let _ = Server::update(sync, frame, publication_outlet, frame_inlet).await;
            });
        }

        {
            let database = database.clone();
            let sync = sync.clone();

            tokio::spawn(async move {
                let _ =
                    Server::listen(listener, database, sync, publication_inlet, frame_outlet).await;
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
        database: Arc<StdMutex<Database>>,
        sync: Arc<TokioMutex<Sync>>,
        publication_inlet: PublicationInlet,
        frame_outlet: FrameOutlet,
    ) {
        let fuse = Fuse::new();

        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let connection: PlainConnection = stream.into();

                let database = database.clone();
                let sync = sync.clone();

                let publication_inlet = publication_inlet.clone();
                let frame_outlet = frame_outlet.clone();

                fuse.spawn(async move {
                    let _ =
                        Server::serve(connection, database, sync, publication_inlet, frame_outlet)
                            .await;
                });
            }
        }
    }

    async fn serve(
        mut connection: PlainConnection,
        database: Arc<StdMutex<Database>>,
        sync: Arc<TokioMutex<Sync>>,
        publication_inlet: PublicationInlet,
        frame_outlet: FrameOutlet,
    ) -> Result<(), Top<ServeError>> {
        let request: Request = connection
            .receive()
            .await
            .pot(ServeError::ConnectionError, here!())?;

        match request {
            Request::LightSubscribe(height) => {
                // This server cannot handle view height values greater than usize::MAX"
                if height <= usize::MAX as u64 {
                    Server::serve_light_subscribe(connection, frame_outlet, height as usize).await
                } else {
                    ServeError::HeightOverflow.fail().spot(here!())
                }
            }

            Request::FullSubscribe => {
                Server::serve_full_subscribe(connection, database, sync).await
            }

            Request::Publish(install) => {
                Server::serve_publish(connection, database, publication_inlet, install).await
            }
            _ => ServeError::UnexpectedRequest.fail().spot(here!()),
        }
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
        database: Arc<StdMutex<Database>>,
        sync: Arc<TokioMutex<Sync>>,
    ) -> Result<(), Top<ServeError>> {
        let (mut receiver, mut local_discovered, mut update_outlet) = {
            let sync = sync.lock().await;

            (
                sync.family.receive(),
                sync.discovered.clone(),
                sync.update_inlet.subscribe(),
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

        // Compute diff between `local_discovered` and `remote_discovered`

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

        // Sort `Install` messages by increasing source height

        updates.sort_by_key(|(height, _)| *height);

        let installs = updates
            .into_iter()
            .map(|(_, install)| install)
            .collect::<Vec<_>>();

        // Send all install messages in a `Vec`

        connection
            .send(&Response::Update(installs))
            .await
            .pot(ServeError::ConnectionError, here!())?;

        // Serve updates

        loop {
            tokio::select! {
                biased;

                update = update_outlet.recv() => {
                    let install = update.map_err(ServeError::update_error).map_err(Doom::into_top).spot(here!())?;

                    connection.send(&Response::Update(vec![install]))
                        .await
                        .pot(ServeError::ConnectionError, here!())?;
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

    async fn serve_publish(
        mut connection: PlainConnection,
        database: Arc<StdMutex<Database>>,
        publication_inlet: PublicationInlet,
        install: Install,
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

                let hash = hash::hash(&install).unwrap();
                database.installs.insert(hash, install.clone());
            } else {
                ServeError::UnknownView.fail().spot(here!())?
            }
        }

        // Because `publication_inlet` is unbounded, this can only fail if the
        // corresponding `publication_outlet` is dropped, in which case the
        // `Server` is shutting down and we don't care about the error
        let _ = publication_inlet.send(install).await;

        connection
            .send(&Response::AcknowledgePublish)
            .await
            .pot(ServeError::ConnectionError, here!())?;

        Ok(())
    }

    async fn update(
        sync: Arc<TokioMutex<Sync>>,
        mut frame: Arc<Frame>,
        mut publication_outlet: PublicationOutlet,
        frame_inlet: FrameInlet,
    ) {
        loop {
            if let Some(install) = publication_outlet.recv().await {
                if let Some(update) = frame.update(install.clone()).await {
                    frame = Arc::new(update);

                    // The corresponding `frame_outlet` is held by listen, so this
                    // never returns an error until the server is shutting down
                    let _ = frame_inlet.send(frame.clone());
                }

                let hash = hash::hash(&install).unwrap();

                let mut sync = sync.lock().await;

                let mut transaction = CollectionTransaction::new();
                let query = transaction.contains(&hash).unwrap();
                let response = sync.discovered.execute(transaction);

                if !response.contains(&query) {
                    let mut transaction = CollectionTransaction::new();
                    transaction.insert(hash).unwrap();
                    sync.discovered.execute(transaction);

                    let _ = sync.update_inlet.send(install);
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

        let server = Server::new(genesis, (Ipv4Addr::UNSPECIFIED, 0), Default::default())
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
                .send(&Request::LightSubscribe(client.current().height() as u64))
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
