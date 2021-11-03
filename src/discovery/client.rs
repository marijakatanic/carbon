use crate::{
    discovery::{ClientSettings, Mode, Request, Response},
    view::{Install, Transition, View},
};

use doomstack::{here, Doom, ResultExt, Top};

use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use talk::crypto::primitives::hash::Hash;
use talk::net::traits::TcpConnect;
use talk::net::{PlainReceiver, PlainSender};
use talk::sync::fuse::Fuse;
use talk::{crypto::primitives::hash, net::PlainConnection};

use tokio::sync::watch::{Receiver, Sender};
use tokio::sync::{watch, Mutex};
use tokio::time;

use zebra::database::{Collection, CollectionTransaction, Family};
use zebra::Commitment;

type TransitionInlet = Sender<Option<Transition>>;
type TransitionOutlet = Receiver<Option<Transition>>;

pub(crate) struct Client {
    server: Box<dyn TcpConnect>,
    database: Arc<Mutex<Database>>,
    transition_outlet: Mutex<TransitionOutlet>,
    settings: ClientSettings,
    _fuse: Fuse,
}

struct Database {
    top: usize,
    views: HashMap<Commitment, View>,
    discovered: Collection<Hash>,
    installs: HashMap<Hash, Install>,
}

#[derive(Doom)]
enum PublishAttemptError {
    #[doom(description("Failed to connect: {}", source))]
    #[doom(wrap(connect_failed))]
    ConnectFailed { source: io::Error },
    #[doom(description("Connection error"))]
    ConnectionError,
    #[doom(description("Unexpected response"))]
    UnexpectedResponse,
}

#[derive(Doom)]
enum SubscribeAttemptError {
    #[doom(description("Failed to connect: {}", source))]
    #[doom(wrap(connect_failed))]
    ConnectFailed { source: io::Error },
    #[doom(description("Error while handshaking"))]
    HandshakeError,
    #[doom(description("Connection error"))]
    ListenFailed,
    #[doom(description("`keep_alive` failed"))]
    KeepAliveFailed,
}

#[derive(Doom)]
enum HandshakeError {
    #[doom(description("Connection error"))]
    ConnectionError,
    #[doom(description("Malformed `Question`"))]
    MalformedQuestion,
}

#[derive(Doom)]
enum ListenError {
    #[doom(description("Connection error"))]
    ConnectionError,
    #[doom(description("Unexpected response"))]
    UnexpectedResponse,
    #[doom(description("Error while acquiring install message"))]
    AcquireError,
}

#[derive(Doom)]
enum KeepAliveError {
    #[doom(description("Connection error"))]
    ConnectionError,
}

#[derive(Doom)]
enum AcquireError {
    #[doom(description("Invalid install message"))]
    InvalidInstall,
    #[doom(description("Unexpected install message"))]
    UnexpectedInstall,
}

impl Client {
    pub(crate) fn new<T>(server: T, genesis: View, settings: ClientSettings) -> Self
    where
        T: 'static + Clone + TcpConnect,
    {
        let top = genesis.height();

        let mut views = HashMap::new();
        views.insert(genesis.identifier(), genesis);

        let family = Family::new();
        let discovered = family.empty_collection();

        let installs = HashMap::new();

        let database = Arc::new(Mutex::new(Database {
            top,
            views,
            discovered,
            installs,
        }));

        let (transition_inlet, transition_outlet) = watch::channel(None);
        let transition_outlet = Mutex::new(transition_outlet);

        let fuse = Fuse::new();

        {
            let server = server.clone();
            let settings = settings.clone();
            let database = database.clone();

            fuse.spawn(async move {
                let _ = Client::subscribe(server, settings, database, transition_inlet).await;
            });
        }

        let server = Box::new(server);

        Client {
            server,
            database,
            transition_outlet,
            settings,
            _fuse: fuse,
        }
    }

    pub(crate) async fn view(&self, identifier: &Commitment) -> Option<View> {
        self.database.lock().await.views.get(identifier).cloned()
    }

    pub(crate) async fn install(&self, hash: &Hash) -> Option<Install> {
        self.database.lock().await.installs.get(hash).cloned()
    }

    pub(crate) async fn next(&self) -> Transition {
        let mut transition_outlet = self.transition_outlet.lock().await;

        // This cannot fail: the corresponding `transition_inlet` is
        // held by `run`, which returns only when `self._fuse` drops:
        // if `self.transition_outlet.changed()` returned an error,
        // `self` would have been dropped, which would make it impossible
        // to call `self.next()`.
        transition_outlet.changed().await.unwrap();

        // This cannot fail, as `attempt` only feeds `Some(..)` into `transition_inlet`.
        let transition = transition_outlet.borrow_and_update().clone().unwrap();

        transition
    }

    pub(crate) async fn beyond(&self, height: usize) -> Transition {
        let mut transition_outlet = self.transition_outlet.lock().await;

        loop {
            if let Some(transition) = &*transition_outlet.borrow_and_update() {
                if transition.destination().height() > height {
                    return transition.clone();
                }
            }

            // This cannot fail (see `next`)
            transition_outlet.changed().await.unwrap();
        }
    }

    pub(crate) async fn publish(&self, install: Install) {
        let mut sleep_agent = self.settings.retry_schedule.agent();

        loop {
            if self.publish_attempt(install.clone()).await.is_ok() {
                return;
            }

            sleep_agent.step().await;
        }
    }

    async fn publish_attempt(&self, install: Install) -> Result<(), Top<PublishAttemptError>> {
        let mut connection = self
            .server
            .connect()
            .await
            .map_err(PublishAttemptError::connect_failed)
            .map_err(Doom::into_top)
            .spot(here!())?;

        connection
            .send(&Request::Publish(install))
            .await
            .pot(PublishAttemptError::ConnectionError, here!())?;

        match connection
            .receive()
            .await
            .pot(PublishAttemptError::ConnectionError, here!())?
        {
            Response::AcknowledgePublish => Ok(()),
            _ => PublishAttemptError::UnexpectedResponse.fail(),
        }
    }

    async fn subscribe<T>(
        server: T,
        settings: ClientSettings,
        database: Arc<Mutex<Database>>,
        mut transition_inlet: TransitionInlet,
    ) where
        T: 'static + TcpConnect,
    {
        let mut sleep_agent = settings.retry_schedule.agent();

        loop {
            let mut progress = false;

            let _ = Client::subscribe_attempt(
                &server,
                &settings,
                &database,
                &mut transition_inlet,
                &mut progress,
            )
            .await;

            if progress {
                sleep_agent = settings.retry_schedule.agent();
            }

            sleep_agent.step().await;
        }
    }

    async fn subscribe_attempt<T>(
        server: &T,
        settings: &ClientSettings,
        database: &Arc<Mutex<Database>>,
        transition_inlet: &mut TransitionInlet,
        progress: &mut bool,
    ) -> Result<(), Top<SubscribeAttemptError>>
    where
        T: 'static + TcpConnect,
    {
        let mut connection = server
            .connect()
            .await
            .map_err(SubscribeAttemptError::connect_failed)
            .map_err(Doom::into_top)
            .spot(here!())?;

        match &settings.mode {
            Mode::Light => {
                Client::light_handshake(database, &mut connection)
                    .await
                    .pot(SubscribeAttemptError::HandshakeError, here!())?;
            }
            Mode::Full => {
                Client::full_handshake(database, &mut connection)
                    .await
                    .pot(SubscribeAttemptError::HandshakeError, here!())?;
            }
        }

        let (sender, receiver) = connection.split();

        let result = tokio::try_join!(
            async {
                Client::keep_alive(sender, settings.keepalive_interval)
                    .await
                    .pot(SubscribeAttemptError::KeepAliveFailed, here!())
            },
            async {
                Client::listen(database, receiver, transition_inlet, progress)
                    .await
                    .pot(SubscribeAttemptError::ListenFailed, here!())
            }
        );

        result.map(|_| ())
    }

    async fn light_handshake(
        database: &Arc<Mutex<Database>>,
        connection: &mut PlainConnection,
    ) -> Result<(), Top<HandshakeError>> {
        let top = database.lock().await.top;

        connection
            .send(&Request::LightSubscribe(top as u64))
            .await
            .pot(HandshakeError::ConnectionError, here!())
    }

    async fn full_handshake(
        database: &Arc<Mutex<Database>>,
        connection: &mut PlainConnection,
    ) -> Result<(), Top<HandshakeError>> {
        connection
            .send(&Request::FullSubscribe)
            .await
            .pot(HandshakeError::ConnectionError, here!())?;

        let discovered = database.lock().await.discovered.clone();
        let mut sender = discovered.send();

        let mut answer = sender.hello();

        loop {
            connection
                .send(&answer)
                .await
                .pot(HandshakeError::ConnectionError, here!())?;

            let question = connection
                .receive()
                .await
                .pot(HandshakeError::ConnectionError, here!())?;

            if let Some(question) = question {
                answer = sender
                    .answer(&question)
                    .pot(HandshakeError::MalformedQuestion, here!())?;
            } else {
                return Ok(());
            }
        }
    }

    async fn listen(
        database: &Arc<Mutex<Database>>,
        mut receiver: PlainReceiver,
        transition_inlet: &mut TransitionInlet,
        progress: &mut bool,
    ) -> Result<(), Top<ListenError>> {
        loop {
            let response = receiver
                .receive()
                .await
                .pot(ListenError::ConnectionError, here!())?;

            match response {
                Response::Update(update) => {
                    Client::acquire(database, transition_inlet, update)
                        .await
                        .pot(ListenError::AcquireError, here!())?;
                }
                Response::KeepAlive => {}
                _ => {
                    // This is also technically misbehaviour
                    return ListenError::UnexpectedResponse.fail();
                }
            }

            *progress = true;
        }
    }

    async fn keep_alive(
        mut sender: PlainSender,
        interval: Duration,
    ) -> Result<(), Top<KeepAliveError>> {
        loop {
            sender
                .send(&Request::KeepAlive)
                .await
                .pot(KeepAliveError::ConnectionError, here!())?;

            time::sleep(interval).await;
        }
    }

    async fn acquire(
        database: &Arc<Mutex<Database>>,
        transition_inlet: &mut TransitionInlet,
        update: Vec<Install>,
    ) -> Result<(), Top<AcquireError>> {
        let mut database = database.lock().await;

        for install in update {
            let transition = install.clone().into_transition().await;

            if database
                .views
                .contains_key(&transition.source().identifier())
            {
                database.views.insert(
                    transition.destination().identifier(),
                    transition.destination().clone(),
                );

                let hash = hash::hash(&install).unwrap();

                let mut transaction = CollectionTransaction::new();

                transaction
                    .insert(hash)
                    .pot(AcquireError::UnexpectedInstall, here!())?;

                database.discovered.execute(transaction).await;
                database.installs.insert(hash, install);

                if transition.destination().height() > database.top {
                    database.top = transition.destination().height();

                    // This fails only if the corresponding `transition_outlet` is dropped,
                    // in which case the whole `Client` is being dropped, and losing
                    // `transition` is irrelevant.
                    let _ = transition_inlet.send(Some(transition));
                }
            } else {
                // This is a sign of misbehaviour: should this error be handled
                // more seriously than just re-establishing the connection?
                return AcquireError::InvalidInstall.fail();
            }
        }

        Ok(())
    }
}
