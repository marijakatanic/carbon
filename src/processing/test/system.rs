use crate::{
    database::Database,
    discovery::{self, Client, Mode, Server},
    processing::{test::TestBroker, Processor},
    view::View,
};

use talk::{crypto::KeyChain, net::test::System as NetSystem};

pub(crate) struct System {
    pub view: View,
    pub discovery_server: Server,
    pub discovery_client: Client,
    pub processors: Vec<(KeyChain, Processor)>,
    pub brokers: Vec<TestBroker>,
}

impl System {
    pub async fn setup(processors: usize, brokers: usize) -> Self {
        let (install_generator, discovery_server, _, mut discovery_clients, _) =
            discovery::test::setup(processors, processors, Mode::Full).await;
        let discovery_client = discovery_clients.next().unwrap();

        let view = install_generator.view(processors);

        let mut processor_keychains = install_generator.keychains.clone();
        processor_keychains.sort_by_key(|keychain| keychain.keycard().identity());

        let mut broker_keychains = (0..brokers).map(|_| KeyChain::random()).collect::<Vec<_>>();
        broker_keychains.sort_by_key(|keychain| keychain.keycard().identity());

        let NetSystem {
            mut connectors,
            mut listeners,
            ..
        } = NetSystem::setup_with_keychains(
            processor_keychains
                .iter()
                .cloned()
                .chain(broker_keychains.iter().cloned()),
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

        let brokers = broker_keychains
            .into_iter()
            .map(|keychain| TestBroker::new(keychain, view.clone(), connectors.remove(0)))
            .collect::<Vec<TestBroker>>();

        System {
            view,
            discovery_server,
            discovery_client,
            brokers,
            processors,
        }
    }
}
