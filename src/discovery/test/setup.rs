use crate::{
    discovery::{Client, ClientSettings, Mode, Server},
    view::test::InstallGenerator,
};

use std::iter;
use std::iter::Iterator;
use std::net::Ipv4Addr;

pub(crate) async fn setup(
    views: usize,
    genesis: usize,
    mode: Mode,
) -> (InstallGenerator, Server, impl Iterator<Item = Client>) {
    let generator = InstallGenerator::new(views);
    let genesis = generator.view(genesis).await;

    let server = Server::new(
        genesis.clone(),
        (Ipv4Addr::UNSPECIFIED, 0),
        Default::default(),
    )
    .await
    .unwrap();

    let address = server.address();

    let clients = iter::repeat_with(move || {
        Client::new(
            genesis.clone(),
            address.clone(),
            ClientSettings {
                mode,
                ..Default::default()
            },
        )
    });

    (generator, server, clients)
}
