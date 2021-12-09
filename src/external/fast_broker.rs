use crate::{
    external::{
        fast_signup_broker::FastSignupBroker,
        parameters::{BrokerParameters, Export, Parameters},
    },
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use log::{error, info};

use std::time::Duration;

use talk::{
    crypto::KeyChain,
    link::rendezvous::{
        Client as RendezvousClient, ClientError as RendezvousClientError, Connector,
    },
    net::traits::TcpConnect,
};

use tokio::time;

pub struct FastBroker {}

#[derive(Doom)]
pub enum FastBrokerError {
    #[doom(description("Fail"))]
    Fail,
}

impl FastBroker {
    pub async fn new<A: 'static + TcpConnect + Clone>(
        rendezvous: A,
        parameters_file: Option<&str>,
    ) -> Result<Self, Top<FastBrokerError>> {
        // Load default parameters if none are specified.
        let BrokerParameters {
            signup_batch_number,
            signup_batch_size,
        } = match parameters_file {
            Some(filename) => {
                Parameters::read(filename)
                    .pot(FastBrokerError::Fail, here!())?
                    .broker
            }
            None => Parameters::default().broker,
        };

        let keychain = KeyChain::random();

        let connector = Connector::new(rendezvous.clone(), keychain.clone(), Default::default());

        info!("Getting shard");

        let client = RendezvousClient::new(rendezvous, Default::default());
        let shard = loop {
            match client.get_shard(0).await {
                Ok(shard) => break shard,
                Err(e) => match e.top() {
                    RendezvousClientError::ShardIncomplete => {
                        info!("Shard still incomplete, sleeping...");
                        time::sleep(Duration::from_millis(500)).await
                    }
                    _ => {
                        error!("Error obtaining first shard view");
                        return FastBrokerError::Fail.fail();
                    }
                },
            }
        };

        info!(
            "Obtained shard! Genesis identities {:?}",
            shard
                .iter()
                .map(|keycard| keycard.identity())
                .collect::<Vec<_>>()
        );

        let view = View::genesis(shard);

        FastSignupBroker::signup(
            view,
            connector,
            signup_batch_number,
            signup_batch_size,
            Default::default(),
        )
        .await;

        Ok(FastBroker {})
    }
}
