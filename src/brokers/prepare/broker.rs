use crate::{brokers::prepare::BrokerSettings, data::Sponge, discovery::Client, view::View};

use doomstack::{here, Doom, ResultExt, Top};

use std::net::SocketAddr;

use talk::{net::Connector, sync::fuse::Fuse};

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
        discovery: Client,
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

        // let request_sponge = Sponge::new(settings.request_sponge_settings);

        let fuse = Fuse::new();

        fuse.spawn(async move {
            Broker::listen().await;
        });

        todo!()
    }

    async fn listen() {}
}
