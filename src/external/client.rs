use crate::{
    brokers::{
        prepare::{Broker as PrepareBroker, BrokerSettings as PrepareBrokerSettings},
        signup::Broker as SignupBroker,
    },
    data::SpongeSettings,
    discovery::Client as DiscoveryClient,
    external::parameters::{BrokerParameters, Export, Parameters},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use log::{error, info};

use std::{net::Ipv4Addr, sync::Arc, time::Duration};

use talk::{
    crypto::KeyChain,
    link::rendezvous::{
        Client as RendezvousClient, ClientError as RendezvousClientError, Connector,
    },
    net::traits::TcpConnect,
};

pub struct Client {}

#[derive(Doom)]
pub enum ClientError {
    #[doom(description("Fail"))]
    Fail,
}

impl Client {
    pub async fn new<A: 'static + TcpConnect + Clone>(
        rendezvous: A,
        parameters_file: Option<&str>,
    ) -> Result<Self, Top<ClientError>> {
        Ok(Client {})
    }
}
