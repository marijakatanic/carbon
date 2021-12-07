use crate::{
    brokers::prepare::{
        broker::Brokerage, broker_settings::BrokerTaskSettings, Broker, BrokerFailure,
    },
    data::{PingBoard, Sponge},
    discovery::Client,
    view::View,
};

use std::sync::Arc;

use talk::{net::SessionConnector, sync::fuse::Fuse};

impl Broker {
    pub(in crate::brokers::prepare::broker) async fn flush(
        discovery: Arc<Client>,
        view: View,
        brokerage_sponge: Arc<Sponge<Brokerage>>,
        ping_board: PingBoard,
        connector: Arc<SessionConnector>,
        settings: BrokerTaskSettings,
    ) {
        let fuse = Fuse::new();

        loop {
            let brokerages = Broker::prepare(brokerage_sponge.flush().await);

            if brokerages.is_empty() {
                continue;
            }

            let discovery = discovery.clone();
            let view = view.clone();
            let connector = connector.clone();
            let ping_board = ping_board.clone();
            let settings = settings.clone();

            fuse.spawn(async move {
                Broker::broker(discovery, view, connector, ping_board, brokerages, settings).await;
            });
        }
    }

    fn prepare(mut brokerages: Vec<Brokerage>) -> Vec<Brokerage> {
        // Sort `brokerages` by requestor

        brokerages.sort_by_key(|brokerage| brokerage.request.id());

        // Deduplicate and fail `brokerages` by requestor

        // The following implementation does not use `Vec::dedup_*` because,
        // in order to fail a duplicate `Brokerage`, it needs to consume
        // its `reduction_inlet` (which mutable references don't allow)
        let mut previous = None;

        brokerages
            .into_iter()
            .filter_map(|brokerage| {
                if Some(brokerage.request.id()) == previous {
                    let _ = brokerage.reduction_inlet.send(Err(BrokerFailure::Throttle));
                    None
                } else {
                    previous = Some(brokerage.request.id());
                    Some(brokerage)
                }
            })
            .collect()
    }
}
