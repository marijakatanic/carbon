use crate::{
    discovery::{Client, ClientSettings, Mode, Server},
    lattice::{Element as LatticeElement, LatticeAgreement},
    view::View,
};

use serde::{Deserialize, Serialize};

use std::iter;
use std::iter::Iterator;
use std::net::Ipv4Addr;
use std::sync::Arc;

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

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone, Debug)]
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

#[tokio::test]
#[ignore]
async fn develop() {
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

    for lattice in lattices.iter_mut() {
        let (decision, _certificate) = lattice.decide().await;
        println!("Works? {:?}", decision);
    }
}
