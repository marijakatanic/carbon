use crate::{
    discovery::{Client, ClientSettings, Mode, Server},
    view::test::InstallGenerator,
};

use std::net::Ipv4Addr;

pub(crate) async fn setup(
    views: usize,
    genesis: usize,
    clients: usize,
    mode: Mode,
) -> (InstallGenerator, Server, Vec<Client>) {
    let generator = InstallGenerator::new(views);
    let genesis = generator.view(genesis).await;

    let server = Server::new(
        genesis.clone(),
        (Ipv4Addr::UNSPECIFIED, 0),
        Default::default(),
    )
    .await
    .unwrap();

    let clients = (0..clients)
        .map(|_| {
            Client::new(
                genesis.clone(),
                server.address(),
                ClientSettings {
                    mode,
                    ..Default::default()
                },
            )
        })
        .collect::<Vec<_>>();

    (generator, server, clients)
}
