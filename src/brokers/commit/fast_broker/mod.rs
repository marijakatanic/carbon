use crate::{commit::CompletionProof, crypto::Identify, data::PingBoard, view::View};

use doomstack::Doom;

use std::sync::Arc;

use talk::{
    link::context::ConnectDispatcher,
    net::{Connector, SessionConnector},
    sync::fuse::Fuse,
};

use tokio::{io, sync::mpsc};

use super::{Broker, Request};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

type RequestInlet = UnboundedSender<(u64, Vec<Request>)>;
type RequestOutlet = UnboundedReceiver<(u64, Vec<Request>)>;

type CompletionInlet = UnboundedSender<Vec<CompletionProof>>;
type CompletionOutlet = UnboundedReceiver<Vec<CompletionProof>>;

pub(crate) struct FastBroker {
    pub _fuse: Fuse,
    pub completion_outlet: CompletionOutlet,
}

#[derive(Doom)]
pub(crate) enum FastBrokerError {
    #[doom(description("Failed to initialize broker: {}", source))]
    #[doom(wrap(initialize_failed))]
    InitializeFailed { source: io::Error },
}

impl FastBroker {
    pub async fn new<C>(view: View, request_outlet: RequestOutlet, connector: C) -> Self
    where
        C: Connector,
    {
        let dispatcher = ConnectDispatcher::new(connector);
        let context = format!("{:?}::processor::commit", view.identifier());
        let connector = Arc::new(SessionConnector::new(dispatcher.register(context)));

        let ping_board = PingBoard::new(&view);

        let fuse = Fuse::new();

        let (inlet, outlet) = mpsc::unbounded_channel();

        for replica in view.members().keys().copied() {
            let ping_board = ping_board.clone();
            let connector = connector.clone();

            fuse.spawn(async move { Broker::ping(ping_board, connector, replica).await });
        }

        {
            let view = view.clone();
            let ping_board = ping_board.clone();
            let connector = connector.clone();
            let inlet = inlet.clone();

            fuse.spawn(async move {
                FastBroker::flush(view, request_outlet, ping_board, connector, inlet).await;
            });
        }

        FastBroker {
            completion_outlet: outlet,
            _fuse: fuse,
        }
    }
}

mod broker;
mod flush;
