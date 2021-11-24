use crate::{
    database::Database,
    processing::{test::TestBroker, Processor},
    view::{test::InstallGenerator, View},
};

use talk::crypto::KeyChain;
use talk::net::test::System as NetSystem;

pub(crate) struct System {
    pub view: View,
    pub processors: Vec<(KeyChain, Processor)>,
    pub brokers: Vec<TestBroker>,
}

impl System {
    pub async fn setup(processors: usize, brokers: usize) -> Self {
        let install_generator = InstallGenerator::new(processors);
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
            brokers,
            processors,
        }
    }
}
