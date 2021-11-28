use crate::{
    brokers::signup::Broker as SignupBroker,
    database::Database,
    discovery::{self, Client, Mode, Server},
    processing::Processor,
    view::View,
};

use std::net::Ipv4Addr;

use talk::{crypto::KeyChain, net::test::System as NetSystem};

pub(crate) struct System {
    pub view: View,
    pub discovery_server: Server,
    pub discovery_client: Client,
    pub processors: Vec<(KeyChain, Processor)>,
    pub signup_brokers: Vec<SignupBroker>,
}

impl System {
    pub async fn setup(processors: usize, brokers: usize) -> Self {
        let (install_generator, discovery_server, _, mut discovery_clients, _) =
            discovery::test::setup(processors, processors, Mode::Full).await;

        let discovery_client = discovery_clients.next().unwrap();
        let view = install_generator.view(processors);

        let mut processor_keychains = install_generator.keychains.clone();
        processor_keychains.sort_by_key(|keychain| keychain.keycard().identity());

        let mut signup_broker_keychains =
            (0..brokers).map(|_| KeyChain::random()).collect::<Vec<_>>();
        signup_broker_keychains.sort_by_key(|keychain| keychain.keycard().identity());

        let NetSystem {
            mut connectors,
            mut listeners,
            ..
        } = NetSystem::setup_with_keychains(
            processor_keychains
                .iter()
                .cloned()
                .chain(signup_broker_keychains.iter().cloned()),
        )
        .await;

        let processors = processor_keychains
            .into_iter()
            .map(|keychain| {
                (
                    keychain.clone(),
                    Processor::new(
                        keychain,
                        view.clone(),
                        Database::new(),
                        connectors.remove(0),
                        listeners.remove(0),
                        Default::default(),
                    ),
                )
            })
            .collect::<Vec<(KeyChain, Processor)>>();

        let mut signup_brokers = Vec::new();

        for _ in signup_broker_keychains {
            signup_brokers.push(
                SignupBroker::new(
                    view.clone(),
                    (Ipv4Addr::LOCALHOST, 0),
                    connectors.remove(0),
                    Default::default(),
                )
                .await
                .unwrap(),
            );
        }

        System {
            view,
            discovery_server,
            discovery_client,
            signup_brokers,
            processors,
        }
    }
}
