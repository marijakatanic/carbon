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

#[tokio::test]
async fn light_pair_publish_then_beyond() {
    let generator = InstallGenerator::new(32);
    let genesis = generator.view(8).await;

    let server = Server::new(
        genesis.clone(),
        (Ipv4Addr::UNSPECIFIED, 0),
        Default::default(),
    )
    .await
    .unwrap();

    let alice = Client::new(
        genesis.clone(),
        server.address(),
        ClientSettings {
            mode: Mode::Light,
            ..Default::default()
        },
    );

    let bob = Client::new(
        genesis,
        server.address(),
        ClientSettings {
            mode: Mode::Light,
            ..Default::default()
        },
    );

    let install = generator.install(8, 10, []).await;
    alice.publish(install).await;

    let transition = bob.beyond(8).await;

    assert_eq!(transition.source().height(), 8);
    assert_eq!(transition.destination().height(), 10);
}

#[tokio::test]
async fn light_pair_cross_publish() {
    let generator = InstallGenerator::new(32);
    let genesis = generator.view(8).await;

    let server = Server::new(
        genesis.clone(),
        (Ipv4Addr::UNSPECIFIED, 0),
        Default::default(),
    )
    .await
    .unwrap();

    let alice = Client::new(
        genesis.clone(),
        server.address(),
        ClientSettings {
            mode: Mode::Light,
            ..Default::default()
        },
    );

    let bob = Client::new(
        genesis,
        server.address(),
        ClientSettings {
            mode: Mode::Light,
            ..Default::default()
        },
    );

    let install = generator.install(8, 10, []).await;
    alice.publish(install).await;

    let install = generator.install(10, 12, []).await;
    bob.publish(install).await;

    let transition = alice.beyond(10).await;

    assert_eq!(transition.source().height(), 10);
    assert_eq!(transition.destination().height(), 12);
}

#[tokio::test]
async fn light_pair_stream_publish() {
    let generator = InstallGenerator::new(32);
    let genesis = generator.view(8).await;

    let server = Server::new(
        genesis.clone(),
        (Ipv4Addr::UNSPECIFIED, 0),
        Default::default(),
    )
    .await
    .unwrap();

    let alice = Client::new(
        genesis.clone(),
        server.address(),
        ClientSettings {
            mode: Mode::Light,
            ..Default::default()
        },
    );

    let bob = Client::new(
        genesis,
        server.address(),
        ClientSettings {
            mode: Mode::Light,
            ..Default::default()
        },
    );

    let install = generator.install(8, 10, []).await;
    alice.publish(install).await;

    let install = generator.install(10, 12, []).await;
    alice.publish(install).await;

    let install = generator.install(12, 14, []).await;
    alice.publish(install).await;

    let install = generator.install(14, 16, []).await;
    alice.publish(install).await;

    let transition = bob.beyond(15).await;

    assert_eq!(transition.source().height(), 14);
    assert_eq!(transition.destination().height(), 16);
}