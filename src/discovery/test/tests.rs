use crate::{
    discovery::{test, Client, Mode, Server},
    view::test::InstallGenerator,
};

async fn setup_single(
    views: usize,
    genesis: usize,
    mode: Mode,
) -> (InstallGenerator, Server, Client) {
    let (generator, server, mut clients) = test::setup(views, genesis, mode).await;
    (generator, server, clients.next().unwrap())
}

async fn setup_pair(
    views: usize,
    genesis: usize,
    mode: Mode,
) -> (InstallGenerator, Server, (Client, Client)) {
    let (generator, server, mut clients) = test::setup(views, genesis, mode).await;

    (
        generator,
        server,
        (clients.next().unwrap(), clients.next().unwrap()),
    )
}

#[tokio::test]
async fn light_single_publish_then_beyond() {
    let (generator, _server, client) = setup_single(32, 8, Mode::Light).await;

    let install = generator.install(8, 10, []).await;
    client.publish(install).await;

    let transition = client.beyond(8).await;

    assert_eq!(transition.source().height(), 8);
    assert_eq!(transition.destination().height(), 10);
}

#[tokio::test]
async fn light_single_adjacent_publishes_then_beyond() {
    let (generator, _server, client) = setup_single(32, 8, Mode::Light).await;

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
    let (generator, _server, client) = setup_single(32, 8, Mode::Light).await;

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
    let (generator, _server, client) = setup_single(32, 8, Mode::Light).await;

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
    let (generator, _server, (alice, bob)) = setup_pair(32, 8, Mode::Light).await;

    let install = generator.install(8, 10, []).await;
    alice.publish(install).await;

    let transition = bob.beyond(8).await;

    assert_eq!(transition.source().height(), 8);
    assert_eq!(transition.destination().height(), 10);
}

#[tokio::test]
async fn light_pair_cross_publish() {
    let (generator, _server, (alice, bob)) = setup_pair(32, 8, Mode::Light).await;

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
    let (generator, _server, (alice, bob)) = setup_pair(32, 8, Mode::Light).await;

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
