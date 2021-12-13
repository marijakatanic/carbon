use crate::{
    brokers::commit::{Broker, BrokerFailure, Brokerage},
    data::{PingBoard, Sponge},
    view::View,
};

use std::sync::Arc;

use talk::{net::SessionConnector, sync::fuse::Fuse};

impl Broker {
    pub(in crate::brokers::commit::broker) async fn flush(
        view: View,
        brokerage_sponge: Arc<Sponge<Brokerage>>,
        ping_board: PingBoard,
        connector: Arc<SessionConnector>,
    ) {
        let fuse = Fuse::new();

        loop {
            // Remark: `brokerage_sponge.flush()` always returns a non-empty
            // `Vec<Brokerage>`. Because `Broker::prepare` only filters `Id`
            // duplicates, it never produces an empty output on a non-empty input.
            let brokerages = Broker::prepare(brokerage_sponge.flush().await);

            let view = view.clone();
            let ping_board = ping_board.clone();
            let connector = connector.clone();

            fuse.spawn(async move {
                Broker::broker(view, ping_board, connector, brokerages).await;
            });
        }
    }

    fn prepare(mut brokerages: Vec<Brokerage>) -> Vec<Brokerage> {
        // Sort `brokerages` by requestor

        brokerages.sort_by_key(|brokerage| brokerage.request.id());

        // Deduplicate and fail `brokerages` by requestor

        // The following implementation does not use `Vec::dedup_*` because
        // duplicate `Brokerage`s must be failed explicitly. If an element
        // of `brokerages` duplicates the previous, its `completion_inlet` is
        // consumed to `send` a `BrokerFailure`, to the appropriate `serve`
        // task, and the element is filtered out of `prepare`'s return.
        let mut previous = None;

        brokerages
            .into_iter()
            .filter_map(|brokerage| {
                if Some(brokerage.request.id()) == previous {
                    let _ = brokerage
                        .completion_inlet
                        .send(Err(BrokerFailure::Throttle));
                    None
                } else {
                    previous = Some(brokerage.request.id());
                    Some(brokerage)
                }
            })
            .collect()
    }
}
