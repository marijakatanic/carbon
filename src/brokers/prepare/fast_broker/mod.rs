use crate::{
    brokers::prepare::{Broker, BrokerFailure, BrokerSettings, BrokerSettingsComponents},
    crypto::Identify,
    data::PingBoard,
    discovery::Client,
    prepare::BatchCommit,
    signup::IdAssignment,
    view::View,
};

use doomstack::{Doom, Top};

use std::sync::Arc;

use talk::{
    crypto::KeyChain,
    link::context::ConnectDispatcher,
    net::{Connector, SessionConnector},
    sync::fuse::Fuse,
};

use tokio::{
    io,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};

type CommitInlet = UnboundedSender<Result<BatchCommit, BrokerFailure>>;
type CommitOutlet = UnboundedReceiver<Result<BatchCommit, BrokerFailure>>;

pub(crate) struct FastBroker {
    _fuse: Fuse,
    pub commit_outlet: CommitOutlet,
}

#[derive(Doom)]
pub(crate) enum BrokerError {
    #[doom(description("Failed to initialize broker: {}", source))]
    #[doom(wrap(initialize_failed))]
    InitializeFailed { source: io::Error },
}

impl FastBroker {
    pub fn new<C>(
        batch_size: usize,
        batch_number: usize,
        single_sign_percentage: usize,
        clients: Vec<(KeyChain, IdAssignment)>,
        discovery: Arc<Client>,
        view: View,
        connector: C,
        settings: BrokerSettings,
    ) -> Result<Self, Top<BrokerError>>
    where
        C: Connector,
    {
        let BrokerSettingsComponents {
            broker: broker_settings,
            ping: ping_settings,
            ..
        } = settings.into_components();

        let dispatcher = ConnectDispatcher::new(connector);
        let context = format!("{:?}::processor::prepare", view.identifier());
        let connector = Arc::new(SessionConnector::new(dispatcher.register(context)));

        let ping_board = PingBoard::new(&view);

        let fuse = Fuse::new();

        for replica in view.members().keys().copied() {
            let ping_board = ping_board.clone();
            let connector = connector.clone();
            let ping_settings = ping_settings.clone();

            fuse.spawn(
                async move { Broker::ping(ping_board, connector, replica, ping_settings).await },
            );
        }

        let (inlet, outlet) = mpsc::unbounded_channel();

        {
            let discovery = discovery.clone();
            let view = view.clone();
            let ping_board = ping_board.clone();
            let connector = connector.clone();

            fuse.spawn(async move {
                FastBroker::flush(
                    batch_size,
                    batch_number,
                    single_sign_percentage,
                    clients,
                    discovery,
                    view,
                    ping_board,
                    connector,
                    broker_settings,
                    inlet,
                )
                .await;
            });
        }

        Ok(FastBroker {
            _fuse: fuse,
            commit_outlet: outlet,
        })
    }
}

mod broker;
mod flush;

#[cfg(test)]
mod tests {
    use crate::{
        brokers::{
            prepare::{BrokerFailure, Inclusion, Request},
            signup::BrokerFailure as SignupBrokerFailure,
            test::System,
        },
        prepare::BatchCommit,
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
            .receive::<Result<Inclusion, BrokerFailure>>()
            .await
            .unwrap()
            .unwrap();

        let reduction_shard = inclusion
            .certify_reduction(&client_keychain, request.prepare())
            .unwrap();

        connection.send(&reduction_shard).await.unwrap();

        let commit = connection
            .receive::<Result<BatchCommit, BrokerFailure>>()
            .await
            .unwrap()
            .unwrap();

        println!("{:?}\n", inclusion);
        println!("{:?}", commit);

        // tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}
