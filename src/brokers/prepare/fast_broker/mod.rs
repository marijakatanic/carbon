use crate::{
    brokers::{
        commit::Request as CommitRequest,
        prepare::{Broker, BrokerSettings, BrokerSettingsComponents},
    },
    crypto::Identify,
    data::PingBoard,
    discovery::Client,
    signup::IdAssignment,
    view::View,
};

use doomstack::{Doom, Top};

use std::sync::Arc;

use talk::{
    crypto::KeyChain,
    link::context::ConnectDispatcher,
    net::{Connector, SessionConnector},
    sync::fuse::Fuse,
};

use tokio::{
    io,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};

type CommitInlet = UnboundedSender<(u64, Vec<CommitRequest>)>;
type CommitOutlet = UnboundedReceiver<(u64, Vec<CommitRequest>)>;

pub(crate) struct FastBroker {
    pub _fuse: Fuse,
    pub commit_outlet: CommitOutlet,
}

#[derive(Doom)]
pub(crate) enum BrokerError {
    #[doom(description("Failed to initialize broker: {}", source))]
    #[doom(wrap(initialize_failed))]
    InitializeFailed { source: io::Error },
}

impl FastBroker {
    pub fn new<C>(
        local_rate: usize,
        batch_size: usize,
        batch_number: usize,
        single_sign_percentage: usize,
        clients: Vec<(KeyChain, IdAssignment)>,
        discovery: Arc<Client>,
        view: View,
        connector: C,
        settings: BrokerSettings,
    ) -> Result<Self, Top<BrokerError>>
    where
        C: Connector,
    {
        let BrokerSettingsComponents {
            broker: broker_settings,
            ping: ping_settings,
            ..
        } = settings.into_components();

        let dispatcher = ConnectDispatcher::new(connector);
        let context = format!("{:?}::processor::prepare", view.identifier());
        let connector = Arc::new(SessionConnector::new(dispatcher.register(context)));

        let ping_board = PingBoard::new(&view);

        let fuse = Fuse::new();

        for replica in view.members().keys().copied() {
            let ping_board = ping_board.clone();
            let connector = connector.clone();
            let ping_settings = ping_settings.clone();

            fuse.spawn(
                async move { Broker::ping(ping_board, connector, replica, ping_settings).await },
            );
        }

        let (inlet, outlet) = mpsc::unbounded_channel();

        {
            let discovery = discovery.clone();
            let view = view.clone();
            let ping_board = ping_board.clone();
            let connector = connector.clone();

            fuse.spawn(async move {
                FastBroker::flush(
                    local_rate,
                    batch_size,
                    batch_number,
                    single_sign_percentage,
                    clients,
                    discovery,
                    view,
                    ping_board,
                    connector,
                    broker_settings,
                    inlet,
                )
                .await;
            });
        }

        Ok(FastBroker {
            _fuse: fuse,
            commit_outlet: outlet,
        })
    }
}

mod broker;
mod flush;
