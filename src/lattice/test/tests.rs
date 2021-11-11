use crate::{
    crypto::Identify,
    discovery::{Client, ClientSettings, Mode, Server},
    lattice::{Element as LatticeElement, LatticeAgreement},
    view::View,
};

use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;
use std::iter::{self, FromIterator, Iterator};
use std::net::Ipv4Addr;
use std::sync::Arc;

use talk::crypto::primitives::hash;
use talk::crypto::primitives::hash::Hash;
use talk::crypto::KeyChain;
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

#[derive(PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Clone, Debug)]
struct Element(u32);

impl LatticeElement for Element {
    fn validate(
        &self,
        _client: &crate::discovery::Client,
        _view: &crate::view::View,
    ) -> Result<(), doomstack::Top<crate::lattice::ElementError>> {
        Ok(())
    }
}

impl Identify for Element {
    fn identifier(&self) -> Hash {
        hash::hash(&self.0).unwrap()
    }
}

async fn lattice_run() {
    let keychains = (0..10).map(|_| KeyChain::random()).collect::<Vec<_>>();
    let genesis = View::genesis(keychains.iter().map(KeyChain::keycard)).await;
    let (_server, clients) = setup_discovery(genesis.clone(), Mode::Full).await;

    let System {
        connectors,
        listeners,
        ..
    } = System::setup_with_keychains(keychains.clone()).await;

    let mut lattices = keychains
        .into_iter()
        .take(10) // Simulate single crash
        .zip(clients)
        .zip(connectors)
        .zip(listeners)
        .map(|(((keychain, client), connector), listener)| {
            LatticeAgreement::<i32, Element>::new(
                genesis.clone(),
                0,
                keychain,
                Arc::new(client),
                connector,
                listener,
            )
        })
        .collect::<Vec<_>>();

    for (proposal, lattice) in lattices.iter_mut().enumerate() {
        let _ = lattice.propose(Element(proposal as u32)).await;
    }

    let mut decisions = Vec::new();
    for lattice in lattices.iter_mut() {
        let (decision, _certificate) = lattice.decide().await;
        decisions.push(decision);
    }

    let mut sets: Vec<_> = decisions
        .into_iter()
        .map(|decision| BTreeSet::from_iter(decision))
        .collect();
    sets.sort_by_key(|set| set.len());
    for window in sets.windows(2) {
        assert!(window[0].is_subset(&window[1]));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
#[ignore]
async fn develop() {
    for i in 0.. {
        println!("Running {}", i);
        lattice_run().await;
    }
}
