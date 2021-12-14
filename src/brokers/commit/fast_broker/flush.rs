use crate::{
    brokers::commit::{FastBroker, Request},
    commit::CompletionProof,
    data::PingBoard,
    view::View,
};

use std::sync::Arc;

use log::info;
use talk::{net::SessionConnector, sync::fuse::Fuse};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

type RequestInlet = UnboundedSender<(u64, Vec<Request>)>;
type RequestOutlet = UnboundedReceiver<(u64, Vec<Request>)>;

type CompletionInlet = UnboundedSender<Vec<CompletionProof>>;
type CompletionOutlet = UnboundedReceiver<Vec<CompletionProof>>;

impl FastBroker {
    pub(in crate::brokers::commit::fast_broker) async fn flush(
        view: View,
        mut request_outlet: RequestOutlet,
        ping_board: PingBoard,
        connector: Arc<SessionConnector>,
        inlet: CompletionInlet,
    ) {
        let fuse = Fuse::new();

        loop {
            // Remark: `brokerage_sponge.flush()` always returns a non-empty
            // `Vec<Brokerage>`. Because `FastBroker::prepare` only filters `Id`
            // duplicates, it never produces an empty output on a non-empty input.
            if let Some((height, requests)) = request_outlet.recv().await {
                let view = view.clone();
                let ping_board = ping_board.clone();
                let connector = connector.clone();

                let inlet = inlet.clone();

                fuse.spawn(async move {
                    info!("Submitting commit {}", height);

                    let result = FastBroker::broker(view, ping_board, connector, requests).await;

                    inlet.send(result).unwrap();
                });
            } else {
                std::future::pending::<()>().await;
            }
        }
    }
}
