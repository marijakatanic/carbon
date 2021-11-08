use crate::lattice::{Element as LatticeElement, LatticeAgreement};
use serde::{Deserialize, Serialize};

use crate::discovery::Mode;
use crate::view::View;

use talk::crypto::KeyChain;
use talk::net::test::System;

use std::sync::Arc;

use crate::discovery::{Client, ClientSettings, Server};

use std::iter;
use std::iter::Iterator;
use std::net::Ipv4Addr;

pub(crate) async fn setup(genesis: View, mode: Mode) -> (Server, impl Iterator<Item = Client>) {
    let server = Server::new(
        genesis.clone(),
        (Ipv4Addr::UNSPECIFIED, 0),
        Default::default(),
    )
    .await
    .unwrap();

    let server_clients = {
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

    (server, server_clients)
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone)]
struct Element {
    my_proposal: usize,
}

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
async fn develop() {
    let mut keychains = (0..4).map(|_| KeyChain::random()).collect::<Vec<_>>();
    let genesis = View::genesis(keychains.iter().map(|keychain| keychain.keycard())).await;
    let (_server, clients) = setup(genesis.clone(), Mode::Full).await;

    let System {
        mut connectors,
        mut listeners,
        keys,
    } = System::setup_with_keychains(keychains.clone()).await;

    let mut lattices: Vec<LatticeAgreement<i32, Element>> = Vec::new();
    for client in clients.take(4) {
        let lattice = LatticeAgreement::<i32, Element>::new(
            genesis.clone(),
            1,
            keychains.remove(0),
            Arc::new(client),
            connectors.remove(0),
            listeners.remove(0),
        );

        lattices.push(lattice);
    }

    for (i, lattice) in lattices.iter_mut().enumerate() {
        let _ = lattice
            .propose(Element {
                my_proposal: i + 1000,
            })
            .await;
    }

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
}
