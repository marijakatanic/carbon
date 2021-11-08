use crate::{
    discovery::{Client, ClientSettings, Mode, Server},
    view::test::InstallGenerator,
};

use std::iter;
use std::iter::Iterator;
use std::net::Ipv4Addr;

use talk::net::test::TcpProxy;

pub(crate) async fn setup(
    views: usize,
    genesis: usize,
    mode: Mode,
) -> (
    InstallGenerator,
    Server,
    TcpProxy,
    impl Iterator<Item = Client>,
    impl Iterator<Item = Client>,
) {
    let generator = InstallGenerator::new(views);
    let genesis = generator.view(genesis).await;

    let server = Server::new(
        genesis.clone(),
        (Ipv4Addr::UNSPECIFIED, 0),
        Default::default(),
    )
    .await
    .unwrap();

    let proxy = TcpProxy::new(server.address()).await;

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

    let proxy_clients = {
        let address = proxy.address();
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

    (generator, server, proxy, server_clients, proxy_clients)
}
