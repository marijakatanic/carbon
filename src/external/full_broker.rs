use crate::{brokers::signup::Broker as SignupBroker, view::View};

use doomstack::{Doom, Top};

use log::{error, info};

use std::{net::Ipv4Addr, time::Duration};

use talk::{
    crypto::KeyChain,
    link::rendezvous::{
        Client as RendezvousClient, ClientError as RendezvousClientError, Connector,
    },
    net::traits::TcpConnect,
};

use tokio::time;

pub struct FullBroker {
    _signup_broker: SignupBroker,
}

#[derive(Doom)]
pub enum FullBrokerError {
    #[doom(description("Fail"))]
    Fail,
}

impl FullBroker {
    pub async fn new<A: 'static + TcpConnect + Clone>(
        rendezvous: A,
    ) -> Result<Self, Top<FullBrokerError>> {
        let keychain = KeyChain::random();

        let connector = Connector::new(rendezvous.clone(), keychain.clone(), Default::default());

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
        let address = (Ipv4Addr::UNSPECIFIED, 0);

        let _signup_broker = SignupBroker::new(view, address, connector, Default::default())
            .await
            .unwrap();

        Ok(FullBroker { _signup_broker })
    }
}
