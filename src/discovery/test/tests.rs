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

    let mut expected_installs = Vec::new();

    let install = generator.install(8, 10, []).await;
    expected_installs.push(install.identifier());
    client.publish(install).await;

    let transition = client.beyond(8).await;

    assert_eq!(transition.source().height(), 8);
    assert_eq!(transition.destination().height(), 10);

    for height in [8, 10] {
        assert!(client
            .view(&generator.view(height).await.identifier())
            .await
            .is_some());
    }

    for install in expected_installs {
        assert!(client.install(&install).await.is_some())
    }
}

#[tokio::test]
async fn light_single_adjacent_publishes_then_beyond() {
    let (generator, _server, client) = setup_single(32, 8, Mode::Light).await;

    let mut expected_installs = Vec::new();

    let install = generator.install(8, 10, []).await;
    expected_installs.push(install.identifier());
    client.publish(install).await;

    let install = generator.install(10, 12, []).await;
    expected_installs.push(install.identifier());
    client.publish(install).await;

    let transition = client.beyond(10).await;

    assert_eq!(transition.source().height(), 10);
    assert_eq!(transition.destination().height(), 12);

    for height in [8, 10, 12] {
        assert!(client
            .view(&generator.view(height).await.identifier())
            .await
            .is_some());
    }

    for install in expected_installs {
        assert!(client.install(&install).await.is_some())
    }
}

#[tokio::test]
async fn light_single_overlapping_publishes_then_beyond() {
    let (generator, _server, client) = setup_single(32, 8, Mode::Light).await;

    let mut expected_installs = Vec::new();

    let install = generator.install(8, 10, []).await;
    client.publish(install).await;

    let install = generator.install(8, 12, []).await;
    expected_installs.push(install.identifier());
    client.publish(install).await;

    let transition = client.beyond(10).await;

    assert_eq!(transition.source().height(), 8);
    assert_eq!(transition.destination().height(), 12);

    for height in [8, 12] {
        assert!(client
            .view(&generator.view(height).await.identifier())
            .await
            .is_some());
    }

    for install in expected_installs {
        assert!(client.install(&install).await.is_some())
    }
}

#[tokio::test]
async fn light_single_redundant_publishes_then_beyond() {
    let (generator, _server, client) = setup_single(32, 8, Mode::Light).await;

    let mut expected_installs = Vec::new();

    let install = generator.install(8, 10, []).await;
    expected_installs.push(install.identifier());
    client.publish(install).await;

    let install = generator.install(10, 12, []).await;
    expected_installs.push(install.identifier());
    client.publish(install).await;

    let install = generator.install(8, 9, []).await;
    client.publish(install).await;

    let transition = client.beyond(10).await;

    assert_eq!(transition.source().height(), 10);
    assert_eq!(transition.destination().height(), 12);

    for height in [8, 10, 12] {
        assert!(client
            .view(&generator.view(height).await.identifier())
            .await
            .is_some());
    }

    for install in expected_installs {
        assert!(client.install(&install).await.is_some())
    }
}

#[tokio::test]
async fn light_pair_publish_then_beyond() {
    let (generator, _server, (alice, bob)) = setup_pair(32, 8, Mode::Light).await;

    let mut expected_installs = Vec::new();

    let install = generator.install(8, 10, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    let transition = bob.beyond(8).await;

    assert_eq!(transition.source().height(), 8);
    assert_eq!(transition.destination().height(), 10);

    for height in [8, 10] {
        assert!(bob
            .view(&generator.view(height).await.identifier())
            .await
            .is_some());
    }

    for install in expected_installs {
        assert!(bob.install(&install).await.is_some())
    }
}

#[tokio::test]
async fn light_pair_cross_publish() {
    let (generator, _server, (alice, bob)) = setup_pair(32, 8, Mode::Light).await;

    let mut expected_installs = Vec::new();
    
    let install = generator.install(8, 10, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    let install = generator.install(10, 12, []).await;
    expected_installs.push(install.identifier());
    bob.publish(install).await;

    let transition = alice.beyond(10).await;

    assert_eq!(transition.source().height(), 10);
    assert_eq!(transition.destination().height(), 12);

    for height in [8, 10, 12] {
        assert!(alice
            .view(&generator.view(height).await.identifier())
            .await
            .is_some());
    }

    for install in expected_installs {
        assert!(bob.install(&install).await.is_some())
    }
}

#[tokio::test]
async fn light_pair_stream_publish() {
    let (generator, _server, (alice, bob)) = setup_pair(32, 8, Mode::Light).await;

    let mut expected_installs = Vec::new();

    let install = generator.install(8, 10, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    let install = generator.install(10, 12, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    let install = generator.install(12, 14, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    let install = generator.install(14, 16, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    let transition = bob.beyond(15).await;

    assert_eq!(transition.source().height(), 14);
    assert_eq!(transition.destination().height(), 16);

    for height in [8, 10, 12, 14, 16] {
        assert!(bob
            .view(&generator.view(height).await.identifier())
            .await
            .is_some());
    }

    for install in expected_installs {
        assert!(bob.install(&install).await.is_some())
    }
}

#[tokio::test]
async fn light_pair_redundant_delayed_join() {
    let (generator, _server, mut clients) = test::setup(32, 8, Mode::Light).await;

    let mut expected_installs = Vec::new();

    let alice = clients.next().unwrap();

    let install = generator.install(8, 10, []).await;
    alice.publish(install).await;

    let install = generator.install(10, 12, []).await;
    alice.publish(install).await;

    let install = generator.install(8, 14, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    alice.beyond(13).await;

    let bob = clients.next().unwrap();
    let transition = bob.beyond(13).await;

    assert_eq!(transition.source().height(), 8);
    assert_eq!(transition.destination().height(), 14);

    for height in [8, 14] {
        assert!(bob
            .view(&generator.view(height).await.identifier())
            .await
            .is_some());
    }

    for install in expected_installs {
        assert!(bob.install(&install).await.is_some())
    }
}
