use crate::{
    brokers::prepare::FastBroker as FastPrepareBroker,
    discovery::Client,
    external::{
        fast_signup_broker::FastSignupBroker,
        parameters::{BrokerParameters, Export, Parameters},
    },
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use log::{error, info};

use std::{sync::Arc, time::Duration};

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
            prepare_batch_size,
            prepare_batch_number,
            prepare_single_sign_percentage,
        } = match parameters_file {
            Some(filename) => {
                Parameters::read(filename)
                    .pot(FastBrokerError::Fail, here!())?
                    .broker
            }
            None => Parameters::default().broker,
        };

        info!("Signup batch number: {}", signup_batch_number);
        info!("Signup batch size: {}", signup_batch_size);
        info!("Prepare batch number: {}", prepare_batch_number);
        info!("Prepare batch size: {}", prepare_batch_size);
        info!("Prepare single sign percentage: {}", prepare_single_sign_percentage);

        let keychain = KeyChain::random();

        info!("Getting shard");

        let client = RendezvousClient::new(rendezvous.clone(), Default::default());
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

        let genesis = View::genesis(shard);

        let connector = Connector::new(rendezvous.clone(), keychain.clone(), Default::default());

        let clients = FastSignupBroker::signup(
            genesis.clone(),
            connector,
            signup_batch_number,
            signup_batch_size,
            Default::default(),
        )
        .await;

        info!("Syncing with other brokers...");

        client.publish_card(keychain.keycard().clone(), Some(1)).await.unwrap();

        let _shard = loop {
            match client.get_shard(1).await {
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

        info!("Synced with other brokers. Initiating prepare phase...");

        let discovery = Arc::new(Client::new(
            genesis.clone(),
            rendezvous.clone(),
            Default::default(),
        ));
        let connector = Connector::new(rendezvous, keychain.clone(), Default::default());

        let FastPrepareBroker {
            mut commit_outlet, ..
        } = FastPrepareBroker::new(
            prepare_batch_size,
            prepare_batch_number,
            prepare_single_sign_percentage,
            clients,
            discovery,
            genesis.clone(),
            connector,
            Default::default(),
        )
        .unwrap();

        for _ in 0..prepare_batch_number {
            let _commit = commit_outlet.recv().await.unwrap();
            // Do something
        }

        info!("Prepare complete!");

        Ok(FastBroker {})
    }
}
