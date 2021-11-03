use crate::{
    discovery::{Client, ClientSettings, Mode, Server},
    view::test::InstallGenerator,
};

use std::net::Ipv4Addr;

#[tokio::test]
async fn light_single_publish_then_beyond() {
    let generator = InstallGenerator::new(30);
    let genesis = generator.view(10).await;

    let server = Server::new(
        genesis.clone(),
        (Ipv4Addr::UNSPECIFIED, 0),
        Default::default(),
    )
    .await
    .unwrap();

    let client = Client::new(
        genesis,
        server.address(),
        ClientSettings {
            mode: Mode::Light,
            ..Default::default()
        },
    );

    let install = generator.install(10, 12, []).await;
    client.publish(install).await;

    let transition = client.beyond(10).await;

    assert_eq!(transition.source().height(), 10);
    assert_eq!(transition.destination().height(), 12);
}
