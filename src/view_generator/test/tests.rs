use crate::{
    churn::{Churn, Resignation},
    crypto::Identify,
    discovery::{Client, ClientSettings, Mode, Server},
    view::{test::InstallGenerator, View},
    view_generator::ViewGenerator,
};

use std::iter::Iterator;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::{collections::BTreeSet, iter};

use talk::net::test::System;

pub(crate) async fn setup_discovery(
    genesis: View,
    mode: Mode,
) -> (Server, impl Iterator<Item = Client>) {
    let server = Server::new(
        genesis.clone(),
        (Ipv4Addr::LOCALHOST, 0),
        Default::default(),
    )
    .await
    .unwrap();

    let clients = {
        let address = server.address();
        let genesis = genesis.clone();

        iter::repeat_with(move || {
            Client::new(
                genesis.clone(),
                address.clone(),
                ClientSettings {
                    mode,
                    ..Default::default()
                },
            )
        })
    };

    (server, clients)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
#[ignore]
async fn stress_simple() {
    const MAX_N: usize = 30;

    let install_gen = InstallGenerator::new(MAX_N);

    let keychains = install_gen.keychains.clone();
    let genesis = install_gen.view(MAX_N - 1);
    let (_server, mut clients) = setup_discovery(genesis.clone(), Mode::Full).await;

    let clients = (0..MAX_N)
        .map(|_| Arc::new(clients.next().unwrap()))
        .collect::<Vec<_>>();

    let client = clients[0].clone();
    let mut install = install_gen.install(MAX_N - 1, MAX_N, []);
    client.publish(install.clone()).await;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let mut resignations = (4..MAX_N)
        .map(|i| Resignation::new(keychains.get(i).unwrap()))
        .rev();

    let mut view = install_gen.view(MAX_N);

    for i in 0..MAX_N - 4 {
        let System {
            mut connectors,
            mut listeners,
            ..
        } = System::setup_with_keychains(keychains.clone()).await;

        let churn = Churn::Resignation(resignations.next().unwrap().into());

        let mut generators = (0..MAX_N - i)
            .map(|j| {
                ViewGenerator::new(
                    view.clone(),
                    keychains[j].clone(),
                    clients[j].clone(),
                    connectors.remove(0),
                    listeners.remove(0),
                )
            })
            .collect::<Vec<_>>();

        println!("PROPOSING");

        let mut the_one = generators.remove(0);
        the_one.propose_churn(install.identifier(), vec![churn]);

        install = the_one.decide().await;

        client.publish(install.clone()).await;

        for client in clients.iter() {
            client.beyond(view.height()).await;
            assert!(client.install(&install.identifier()).is_some());
        }

        view = install.clone().into_transition().destination().clone();

        assert_eq!(
            view.members().keys().cloned().collect::<BTreeSet<_>>(),
            keychains[0..MAX_N - i - 1]
                .iter()
                .map(|keychain| keychain.keycard().identity())
                .collect::<BTreeSet<_>>()
        );

        println!(
            "VIEW HEIGHT: {}, VIEW: {:?}",
            view.height(),
            view.identifier()
        );
    }

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}
