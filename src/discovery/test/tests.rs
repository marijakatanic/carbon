use crate::{
    discovery::{test, Client, Mode, Server},
    view::test::InstallGenerator,
};

use std::time::Duration;

use talk::net::test::TcpProxy;

use tokio::time;

async fn setup_single(
    views: usize,
    genesis: usize,
    mode: Mode,
) -> (InstallGenerator, Server, TcpProxy, Client) {
    let (generator, server, proxy, mut server_clients, _) = test::setup(views, genesis, mode).await;
    (generator, server, proxy, server_clients.next().unwrap())
}

async fn setup_pair(
    views: usize,
    genesis: usize,
    mode: Mode,
) -> (InstallGenerator, Server, TcpProxy, (Client, Client)) {
    let (generator, server, proxy, mut server_clients, _) = test::setup(views, genesis, mode).await;

    (
        generator,
        server,
        proxy,
        (
            server_clients.next().unwrap(),
            server_clients.next().unwrap(),
        ),
    )
}

#[tokio::test]
async fn light_single_publish_then_beyond() {
    let (generator, _server, _proxy, client) = setup_single(32, 8, Mode::Light).await;

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
    let (generator, _server, _proxy, client) = setup_single(32, 8, Mode::Light).await;

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
    let (generator, _server, _proxy, client) = setup_single(32, 8, Mode::Light).await;

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
    let (generator, _server, _proxy, client) = setup_single(32, 8, Mode::Light).await;

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
    let (generator, _server, _proxy, (alice, bob)) = setup_pair(32, 8, Mode::Light).await;

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
    let (generator, _server, _proxy, (alice, bob)) = setup_pair(32, 8, Mode::Light).await;

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
        assert!(alice.install(&install).await.is_some())
    }
}

#[tokio::test]
async fn light_pair_stream_publish() {
    let (generator, _server, _proxy, (alice, bob)) = setup_pair(32, 8, Mode::Light).await;

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
    let (generator, _server, _proxy, mut server_clients, _) = test::setup(32, 8, Mode::Light).await;

    let mut expected_installs = Vec::new();
    let mut excluded_installs = Vec::new();

    let alice = server_clients.next().unwrap();

    let install = generator.install(8, 10, []).await;
    excluded_installs.push(install.identifier());
    alice.publish(install).await;

    let install = generator.install(10, 12, []).await;
    excluded_installs.push(install.identifier());
    alice.publish(install).await;

    let install = generator.install(8, 14, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    alice.beyond(13).await;

    let bob = server_clients.next().unwrap();
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

    for install in excluded_installs {
        assert!(bob.install(&install).await.is_none())
    }
}

#[tokio::test]
async fn full_single_publish_then_beyond() {
    let (generator, _server, _proxy, client) = setup_single(32, 8, Mode::Full).await;

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
async fn full_single_adjacent_publishes_then_beyond() {
    let (generator, _server, _proxy, client) = setup_single(32, 8, Mode::Full).await;

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
async fn full_single_overlapping_publishes_then_beyond() {
    let (generator, _server, _proxy, client) = setup_single(32, 8, Mode::Full).await;

    let mut expected_installs = Vec::new();

    let install = generator.install(8, 10, []).await;
    expected_installs.push(install.identifier());
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
async fn full_single_redundant_publishes_then_beyond() {
    let (generator, _server, _proxy, client) = setup_single(32, 8, Mode::Full).await;

    let mut expected_installs = Vec::new();

    let install = generator.install(8, 10, []).await;
    expected_installs.push(install.identifier());
    client.publish(install).await;

    let install = generator.install(10, 12, []).await;
    expected_installs.push(install.identifier());
    client.publish(install).await;

    let install = generator.install(8, 9, []).await;
    expected_installs.push(install.identifier());
    client.publish(install).await;

    let install = generator.install(12, 13, []).await;
    expected_installs.push(install.identifier());
    client.publish(install).await;

    let transition = client.beyond(12).await;

    assert_eq!(transition.source().height(), 12);
    assert_eq!(transition.destination().height(), 13);

    for height in [8, 10, 12, 13] {
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
async fn full_pair_publish_then_beyond() {
    let (generator, _server, _proxy, (alice, bob)) = setup_pair(32, 8, Mode::Full).await;

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
async fn full_pair_cross_publish() {
    let (generator, _server, _proxy, (alice, bob)) = setup_pair(32, 8, Mode::Full).await;

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
        assert!(alice.install(&install).await.is_some())
    }
}

#[tokio::test]
async fn full_pair_stream_publish() {
    let (generator, _server, _proxy, (alice, bob)) = setup_pair(32, 8, Mode::Full).await;

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
async fn full_pair_redundant_delayed_join() {
    let (generator, _server, _proxy, mut server_clients, _) = test::setup(32, 8, Mode::Full).await;

    let mut expected_installs = Vec::new();

    let alice = server_clients.next().unwrap();

    let install = generator.install(8, 10, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    let install = generator.install(10, 12, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    let install = generator.install(8, 14, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    alice.beyond(13).await;

    let bob = server_clients.next().unwrap();
    let transition = bob.beyond(13).await;

    assert_eq!(transition.source().height(), 8);
    assert_eq!(transition.destination().height(), 14);

    for height in [8, 10, 12, 14] {
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
#[ignore]
async fn full_pair_double_sync() {
    let (generator, _server, mut proxy, mut server_clients, mut proxy_clients) =
        test::setup(32, 8, Mode::Full).await;

    let mut expected_installs = Vec::new();

    let alice = server_clients.next().unwrap();
    let bob = proxy_clients.next().unwrap();

    let install = generator.install(8, 10, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    let install = generator.install(10, 12, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    let install = generator.install(12, 14, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    bob.beyond(12).await;

    proxy.stop().await;

    let install = generator.install(14, 16, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    let install = generator.install(16, 18, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    let install = generator.install(18, 20, []).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    proxy.reset().await;

    proxy.start().await;

    let transition = bob.beyond(18).await;

    assert_eq!(transition.source().height(), 18);
    assert_eq!(transition.destination().height(), 20);

    for height in [8, 10, 12, 14, 16, 18, 20] {
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
#[ignore]
async fn full_pair_double_sync_server_lag() {
    let (generator, _server, mut proxy, mut server_clients, mut proxy_clients) =
        test::setup(32, 8, Mode::Full).await;

    let mut expected_installs = Vec::new();

    let alice = server_clients.next().unwrap();
    let bob = proxy_clients.next().unwrap();

    let install = generator.install(8, 10, [11]).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    let install = generator.install(10, 12, [13]).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    let install = generator.install(12, 14, [15]).await;
    expected_installs.push(install.identifier());
    alice.publish(install).await;

    bob.beyond(12).await;

    proxy.stop().await;

    let address = _server.address();
    drop(_server);

    time::sleep(Duration::from_millis(500)).await;

    let _server = Server::new(generator.view(8).await, address, Default::default())
        .await
        .unwrap();

    let charlie = Client::new(generator.view(8).await, address, Default::default());

    let install = generator.install(8, 16, []).await;
    charlie.publish(install).await;

    proxy.reset().await;
    proxy.start().await;

    let transition = bob.beyond(15).await;

    assert_eq!(transition.source().height(), 8);
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
