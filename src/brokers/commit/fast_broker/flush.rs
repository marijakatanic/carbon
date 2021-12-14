use crate::{
    brokers::commit::{FastBroker, Request},
    data::PingBoard,
    view::View,
};

use std::sync::Arc;

use talk::{net::SessionConnector, sync::fuse::Fuse};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

type RequestInlet = UnboundedSender<Vec<Request>>;
type RequestOutlet = UnboundedReceiver<Vec<Request>>;

impl FastBroker {
    pub(in crate::brokers::commit::fast_broker) async fn flush(
        view: View,
        mut request_outlet: RequestOutlet,
        ping_board: PingBoard,
        connector: Arc<SessionConnector>,
    ) {
        let fuse = Fuse::new();

        loop {
            // Remark: `brokerage_sponge.flush()` always returns a non-empty
            // `Vec<Brokerage>`. Because `FastBroker::prepare` only filters `Id`
            // duplicates, it never produces an empty output on a non-empty input.
            let requests = request_outlet.recv().await.unwrap();

            let view = view.clone();
            let ping_board = ping_board.clone();
            let connector = connector.clone();

            fuse.spawn(async move {
                FastBroker::broker(view, ping_board, connector, requests).await;
            });
        }
    }
}
