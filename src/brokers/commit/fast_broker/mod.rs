use crate::{crypto::Identify, data::PingBoard, view::View};

use doomstack::{Doom, Top};

use std::sync::Arc;

use talk::{
    link::context::ConnectDispatcher,
    net::{Connector, SessionConnector},
    sync::fuse::Fuse,
};

use tokio::{io, net::ToSocketAddrs};

use super::{Broker, Request};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

type RequestInlet = UnboundedSender<Vec<Request>>;
type RequestOutlet = UnboundedReceiver<Vec<Request>>;

pub(crate) struct FastBroker {
    _fuse: Fuse,
}

#[derive(Doom)]
pub(crate) enum FastBrokerError {
    #[doom(description("Failed to initialize broker: {}", source))]
    #[doom(wrap(initialize_failed))]
    InitializeFailed { source: io::Error },
}

impl FastBroker {
    pub async fn new<A, C>(
        view: View,
        request_outlet: RequestOutlet,
        connector: C,
    ) -> Result<Self, Top<FastBrokerError>>
    where
        A: ToSocketAddrs,
        C: Connector,
    {
        let dispatcher = ConnectDispatcher::new(connector);
        let context = format!("{:?}::processor::commit", view.identifier());
        let connector = Arc::new(SessionConnector::new(dispatcher.register(context)));

        let ping_board = PingBoard::new(&view);

        let fuse = Fuse::new();

        for replica in view.members().keys().copied() {
            let ping_board = ping_board.clone();
            let connector = connector.clone();

            fuse.spawn(async move { Broker::ping(ping_board, connector, replica).await });
        }

        {
            let view = view.clone();
            let ping_board = ping_board.clone();
            let connector = connector.clone();

            fuse.spawn(async move {
                FastBroker::flush(view, request_outlet, ping_board, connector).await;
            });
        }

        Ok(FastBroker { _fuse: fuse })
    }
}

mod broker;
mod flush;
