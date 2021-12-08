use crate::{external::fast_signup_broker::FastSignupBroker, view::View};

use doomstack::{Doom, Top};

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
pub enum FullBrokerError {
    #[doom(description("Fail"))]
    Fail,
}

impl FastBroker {
    pub async fn new<A: 'static + TcpConnect + Clone>(
        rendezvous: A,
    ) -> Result<Self, Top<FullBrokerError>> {
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
                        return FullBrokerError::Fail.fail();
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

        FastSignupBroker::signup(view, connector, Default::default()).await;

        Ok(FastBroker {})
    }
}
