use crate::{
    brokers::prepare::{BrokerSettings, Request},
    crypto::Identify,
    data::Sponge,
    discovery::Client,
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use std::{net::SocketAddr, sync::Arc};

use talk::{
    link::context::ConnectDispatcher,
    net::{Connector, SessionConnector},
    sync::fuse::Fuse,
};

use tokio::{
    io,
    net::{TcpListener, ToSocketAddrs},
};

pub(crate) struct Broker {
    address: SocketAddr,
    _fuse: Fuse,
}

#[derive(Doom)]
pub(crate) enum BrokerError {
    #[doom(description("Failed to initialize broker: {}", source))]
    #[doom(wrap(initialize_failed))]
    InitializeFailed { source: io::Error },
}

impl Broker {
    pub async fn new<A, C>(
        _discovery: Client,
        view: View,
        address: A,
        connector: C,
        settings: BrokerSettings,
    ) -> Result<Self, Top<BrokerError>>
    where
        A: ToSocketAddrs,
        C: Connector,
    {
        let listener = TcpListener::bind(address)
            .await
            .map_err(BrokerError::initialize_failed)
            .map_err(Doom::into_top)
            .spot(here!())?;

        let address = listener
            .local_addr()
            .map_err(BrokerError::initialize_failed)
            .map_err(Doom::into_top)
            .spot(here!())?;

        let dispatcher = ConnectDispatcher::new(connector);
        let context = format!("{:?}::processor::prepare", view.identifier());
        let _connector = Arc::new(SessionConnector::new(dispatcher.register(context)));

        let _request_sponge = Sponge::<Request>::new(settings.request_sponge_settings);

        let fuse = Fuse::new();

        fuse.spawn(async move {
            Broker::listen().await;
        });

        Ok(Broker {
            address,
            _fuse: fuse,
        })
    }

    async fn listen() {}
}
