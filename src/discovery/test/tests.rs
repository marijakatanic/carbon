use crate::{
    discovery::{Client, ClientSettings, Mode, Server},
    view::test::InstallGenerator,
};

use std::net::Ipv4Addr;

#[tokio::test]
async fn light_single_publish_then_beyond() {
    let generator = InstallGenerator::new(32);
    let genesis = generator.view(8).await;

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

    let install = generator.install(8, 10, []).await;
    client.publish(install).await;

    let transition = client.beyond(8).await;

    assert_eq!(transition.source().height(), 8);
    assert_eq!(transition.destination().height(), 10);
}

#[tokio::test]
async fn light_single_adjacent_publishes_then_beyond() {
    let generator = InstallGenerator::new(32);
    let genesis = generator.view(8).await;

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

    let install = generator.install(8, 10, []).await;
    client.publish(install).await;

    let install = generator.install(10, 12, []).await;
    client.publish(install).await;

    let transition = client.beyond(10).await;

    assert_eq!(transition.source().height(), 10);
    assert_eq!(transition.destination().height(), 12);
}

#[tokio::test]
async fn light_single_overlapping_publishes_then_beyond() {
    let generator = InstallGenerator::new(32);
    let genesis = generator.view(8).await;

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

    let install = generator.install(8, 10, []).await;
    client.publish(install).await;

    let install = generator.install(8, 12, []).await;
    client.publish(install).await;

    let transition = client.beyond(10).await;

    assert_eq!(transition.source().height(), 8);
    assert_eq!(transition.destination().height(), 12);
}

#[tokio::test]
async fn light_single_redundant_publishes_then_beyond() {
    let generator = InstallGenerator::new(32);
    let genesis = generator.view(8).await;

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

    let install = generator.install(8, 10, []).await;
    client.publish(install).await;

    let install = generator.install(10, 12, []).await;
    client.publish(install).await;

    let install = generator.install(8, 9, []).await;
    client.publish(install).await;

    let transition = client.beyond(10).await;

    assert_eq!(transition.source().height(), 10);
    assert_eq!(transition.destination().height(), 12);
}