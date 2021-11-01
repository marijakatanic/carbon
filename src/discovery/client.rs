use crate::{
    discovery::{ClientSettings, Request, Response},
    view::{Install, Transition, View},
};

use doomstack::{here, Doom, ResultExt, Top};

use std::collections::HashMap;
use std::io;
use std::time::Duration;

use talk::net::traits::TcpConnect;
use talk::net::PlainSender;
use talk::sync::fuse::{Fuse, Relay};

use tokio::sync::watch;
use tokio::sync::watch::{Receiver, Sender};
use tokio::time;

use zebra::Commitment;

type TransitionInlet = Sender<Option<Transition>>;
type TransitionOutlet = Receiver<Option<Transition>>;

pub(crate) struct Client {
    server: Box<dyn TcpConnect>,
    transition_outlet: TransitionOutlet,
    settings: ClientSettings,
    _fuse: Fuse,
}

struct Database {
    top: usize,
    views: HashMap<Commitment, View>,
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
enum SubscribeError {
    #[doom(description("`subscribe` interrupted"))]
    SubscribeInterrupted,
}

#[derive(Doom)]
enum SubscribeAttemptError {
    #[doom(description("`subscribe_attempt` interrupted"))]
    SubscribeAttemptInterrupted,
    #[doom(description("Failed to connect: {}", source))]
    #[doom(wrap(connect_failed))]
    ConnectFailed { source: io::Error },
    #[doom(description("Connection error"))]
    ConnectionError { progress: bool },
    #[doom(description("Unexpected response"))]
    UnexpectedResponse { progress: bool },
    #[doom(description("Invalid install message"))]
    InvalidInstall { progress: bool },
}

#[derive(Doom)]
enum KeepAliveError {
    #[doom(description("`keep_alive` interrupted"))]
    KeepAliveInterrupted,
    #[doom(description("Connection error"))]
    ConnectionError,
}

impl Client {
    pub(crate) fn new<T>(server: T, genesis: View, settings: ClientSettings) -> Self
    where
        T: 'static + Clone + TcpConnect,
    {
        let top = genesis.height();

        let mut views = HashMap::new();
        views.insert(genesis.identifier(), genesis);

        let database = Database { top, views };

        let (transition_inlet, transition_outlet) = watch::channel(None);

        let fuse = Fuse::new();

        {
            let server = server.clone();
            let settings = settings.clone();

            let relay = fuse.relay();

            tokio::spawn(async move {
                let _ =
                    Client::subscribe(server, settings, database, transition_inlet, relay).await;
            });
        }

        let server = Box::new(server);

        Client {
            server,
            transition_outlet,
            settings,
            _fuse: fuse,
        }
    }

    pub(crate) async fn next(&mut self) -> Transition {
        // This cannot fail: the corresponding `transition_inlet` is
        // held by `run`, which returns only when `self._fuse` drops:
        // if `self.transition_outlet.changed()` returned an error,
        // `self` would have been dropped, which would make it impossible
        // to call `self.next()`.
        self.transition_outlet.changed().await.unwrap();

        // This cannot fail, as `attempt` only feeds `Some(..)` into `transition_inlet`.
        self.transition_outlet.borrow().clone().unwrap()
    }

    pub(crate) async fn beyond(&mut self, height: usize) -> Transition {
        loop {
            if let Some(transition) = &*self.transition_outlet.borrow() {
                if transition.destination().height() > height {
                    return transition.clone();
                }
            }

            // This cannot fail (see `next`)
            self.transition_outlet.changed().await.unwrap();
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
        mut database: Database,
        mut transition_inlet: TransitionInlet,
        mut relay: Relay,
    ) -> Result<(), Top<SubscribeError>>
    where
        T: 'static + TcpConnect,
    {
        let mut sleep_agent = settings.retry_schedule.agent();

        loop {
            let error = Client::subscribe_attempt(
                &server,
                &settings,
                &mut database,
                &mut transition_inlet,
                &mut relay,
            )
            .await
            .unwrap_err();

            match error.top() {
                SubscribeAttemptError::SubscribeAttemptInterrupted => {
                    return Err(error.pot(SubscribeError::SubscribeInterrupted, here!()));
                }
                error => {
                    if error.progress() {
                        sleep_agent = settings.retry_schedule.agent();
                    }
                }
            }

            sleep_agent.step().await;
        }
    }

    async fn subscribe_attempt<T>(
        server: &T,
        settings: &ClientSettings,
        database: &mut Database,
        transition_inlet: &mut TransitionInlet,
        relay: &mut Relay,
    ) -> Result<(), Top<SubscribeAttemptError>>
    where
        T: 'static + TcpConnect,
    {
        let mut progress = false;

        let mut connection = relay
            .map(server.connect())
            .await
            .pot(SubscribeAttemptError::SubscribeAttemptInterrupted, here!())?
            .map_err(SubscribeAttemptError::connect_failed)
            .map_err(Doom::into_top)
            .spot(here!())?;

        relay
            .map(connection.send(&Request::Subscribe(database.top as u64)))
            .await
            .pot(SubscribeAttemptError::SubscribeAttemptInterrupted, here!())?
            .pot(SubscribeAttemptError::ConnectionError { progress }, here!())?;

        let (sender, mut receiver) = connection.split();

        let fuse = Fuse::new();

        {
            let interval = settings.keepalive_interval;
            let relay = fuse.relay();

            tokio::spawn(async move {
                let _ = Client::keep_alive(sender, interval, relay).await;
            });
        }

        loop {
            let response = relay
                .map(receiver.receive())
                .await
                .pot(SubscribeAttemptError::SubscribeAttemptInterrupted, here!())?
                .pot(SubscribeAttemptError::ConnectionError { progress }, here!())?;

            match response {
                Response::Update(update) => {
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
                            return SubscribeAttemptError::InvalidInstall { progress }.fail();
                        }
                    }
                }
                Response::KeepAlive => {}
                _ => {
                    // This is also technically misbehaviour
                    return SubscribeAttemptError::UnexpectedResponse { progress }.fail();
                }
            }

            progress = true;
        }
    }

    async fn keep_alive(
        mut sender: PlainSender,
        interval: Duration,
        mut relay: Relay,
    ) -> Result<(), Top<KeepAliveError>> {
        loop {
            relay
                .map(sender.send(&Request::KeepAlive))
                .await
                .pot(KeepAliveError::KeepAliveInterrupted, here!())?
                .pot(KeepAliveError::ConnectionError, here!())?;

            relay
                .map(time::sleep(interval)) // TODO: Add settings
                .await
                .pot(KeepAliveError::KeepAliveInterrupted, here!())?;
        }
    }
}

impl SubscribeAttemptError {
    fn progress(&self) -> bool {
        match self {
            SubscribeAttemptError::SubscribeAttemptInterrupted
            | SubscribeAttemptError::ConnectFailed { .. } => false,
            SubscribeAttemptError::ConnectionError { progress }
            | SubscribeAttemptError::UnexpectedResponse { progress }
            | SubscribeAttemptError::InvalidInstall { progress } => *progress,
        }
    }
}
