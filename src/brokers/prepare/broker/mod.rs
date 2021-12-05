use crate::{
    brokers::prepare::{BrokerSettings, Brokerage, Failure, PingBoard, Reduction},
    crypto::Identify,
    data::Sponge,
    discovery::Client,
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use std::{net::SocketAddr, sync::Arc};

use talk::{
    link::context::ConnectDispatcher,
    net::{Connector, SessionConnector},
    sync::fuse::Fuse,
};

use tokio::{
    io,
    net::{TcpListener, ToSocketAddrs},
    sync::oneshot::{Receiver, Sender},
};

type ReductionInlet = Sender<Result<Reduction, Failure>>;
type ReductionOutlet = Receiver<Result<Reduction, Failure>>;

pub(crate) struct Broker {
    address: SocketAddr,
    _fuse: Fuse,
}

#[derive(Doom)]
pub(crate) enum BrokerError {
    #[doom(description("Failed to initialize broker: {}", source))]
    #[doom(wrap(initialize_failed))]
    InitializeFailed { source: io::Error },
}

impl Broker {
    pub async fn new<A, C>(
        discovery: Arc<Client>,
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
        let context = format!("{:?}::processor::prepare", view.identifier());
        let connector = Arc::new(SessionConnector::new(dispatcher.register(context)));

        let brokerage_sponge = Arc::new(Sponge::new(settings.brokerage_sponge_settings));
        let ping_board = PingBoard::new(&view);

        let reduction_timeout = settings.reduction_timeout;
        let ping_interval = settings.ping_interval;

        let fuse = Fuse::new();

        {
            let brokerage_sponge = brokerage_sponge.clone();

            fuse.spawn(async move {
                Broker::listen(discovery, brokerage_sponge, listener).await;
            });
        }

        {
            let view = view.clone();
            let connector = connector.clone();

            fuse.spawn(async move {
                Broker::flush(view, brokerage_sponge, connector, reduction_timeout).await;
            });
        }

        for replica in view.members().keys().copied() {
            let ping_board = ping_board.clone();
            let connector = connector.clone();

            fuse.spawn(
                async move { Broker::ping(ping_board, connector, replica, ping_interval).await },
            );
        }

        Ok(Broker {
            address,
            _fuse: fuse,
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }
}

mod broker;
mod flush;
mod frontend;
mod orchestrate;
mod ping;

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        brokers::{
            prepare::{Inclusion, Request},
            signup::BrokerFailure as SignupBrokerFailure,
            test::System,
        },
        signup::{IdAssignment, IdRequest, SignupSettings},
    };

    use talk::{
        crypto::{primitives::hash, KeyChain},
        net::PlainConnection,
    };

    use tokio::net::TcpStream;

    #[tokio::test]
    async fn develop() {
        let System {
            view,
            discovery_server: _discovery_server,
            discovery_client: _discovery_client,
            processors,
            mut signup_brokers,
            mut prepare_brokers,
        } = System::setup(4, 1, 1).await;

        let client_keychain = KeyChain::random();

        // Signup

        let signup_broker = signup_brokers.remove(0);
        let allocator_identity = processors[0].0.keycard().identity();

        let request = IdRequest::new(
            &client_keychain,
            &view,
            allocator_identity,
            SignupSettings::default().work_difficulty,
        );

        let stream = TcpStream::connect(signup_broker.address()).await.unwrap();
        let mut connection: PlainConnection = stream.into();

        connection.send(&request).await.unwrap();

        let assignment = connection
            .receive::<Result<IdAssignment, SignupBrokerFailure>>()
            .await
            .unwrap()
            .unwrap();

        // Prepare

        let prepare_broker = prepare_brokers.remove(0);
        let request = Request::new(&client_keychain, assignment, 0, hash::hash(&42u32).unwrap());

        let stream = TcpStream::connect(prepare_broker.address()).await.unwrap();
        let mut connection: PlainConnection = stream.into();

        connection.send(&request).await.unwrap();

        let inclusion = connection
            .receive::<Result<Inclusion, Failure>>()
            .await
            .unwrap()
            .unwrap();

        let reduction_shard = inclusion
            .certify_reduction(&client_keychain, request.prepare())
            .unwrap();

        connection.send(&reduction_shard).await.unwrap();

        // tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}
