use crate::{
    brokers::{
        prepare::{Broker as PrepareBroker, BrokerSettings as PrepareBrokerSettings},
        signup::Broker as SignupBroker,
    },
    data::SpongeSettings,
    discovery::Client,
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

use tokio::time;

pub struct FullBroker {
    _signup_broker: SignupBroker,
    _prepare_broker: PrepareBroker,
}

#[derive(Doom)]
pub enum FullBrokerError {
    #[doom(description("Fail"))]
    Fail,
}

impl FullBroker {
    pub async fn new<A: 'static + TcpConnect + Clone>(
        rendezvous: A,
        parameters_file: Option<&str>,
        rate: usize,
    ) -> Result<Self, Top<FullBrokerError>> {
        // Load default parameters if none are specified.
        let BrokerParameters {
            signup_batch_size,
            prepare_batch_size,
            prepare_single_sign_percentage,
            brokerage_timeout,
            reduction_timeout,
            ..
        } = match parameters_file {
            Some(filename) => {
                Parameters::read(filename)
                    .pot(FullBrokerError::Fail, here!())?
                    .broker
            }
            None => Parameters::default().broker,
        };

        info!("Rate limit: {}", rate);
        info!("Signup batch size: {}", signup_batch_size);
        info!("Prepare batch size: {}", prepare_batch_size);
        info!("Brokerage timeout: {}", brokerage_timeout);
        info!("Reduction timeout: {}", reduction_timeout);
        let reduction_threshold = 100 - prepare_single_sign_percentage;
        info!(
            "Reduction percentage: {}",
            reduction_threshold
        );

        let signup_keychain = KeyChain::random();
        let prepare_keychain = KeyChain::random();

        let client = RendezvousClient::new(rendezvous.clone(), Default::default());

        info!("Getting shard");

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

        let genesis = View::genesis(shard);
        let connector = Connector::new(
            rendezvous.clone(),
            signup_keychain.clone(),
            Default::default(),
        );
        let address = (Ipv4Addr::UNSPECIFIED, 0);

        let _signup_broker =
            SignupBroker::new(genesis.clone(), address, connector, Default::default())
                .await
                .unwrap();

        let port = _signup_broker.address().port();
        client
            .advertise_port(signup_keychain.keycard().identity(), port)
            .await;
        // So the client can connect
        client
            .publish_card(signup_keychain.keycard().clone(), Some(2))
            .await
            .unwrap();

        info!("Initializing prepare broker...");

        let discovery = Arc::new(Client::new(
            genesis.clone(),
            rendezvous.clone(),
            Default::default(),
        ));
        let connector = Connector::new(rendezvous, prepare_keychain.clone(), Default::default());

        let sponge_settings = SpongeSettings {
            capacity: prepare_batch_size,
            timeout: Duration::from_millis(brokerage_timeout as u64),
        };

        let broker_settings = PrepareBrokerSettings {
            brokerage_sponge_settings: sponge_settings,
            reduction_threshold: reduction_threshold as f64 / 100 as f64,
            reduction_timeout: Duration::from_millis(reduction_timeout as u64),
            optimistic_witness_timeout: Duration::from_secs(1),
            ping_interval: Duration::from_secs(60),
        };

        let address = (Ipv4Addr::UNSPECIFIED, 0);

        let mut _prepare_broker = PrepareBroker::new(
            discovery,
            genesis.clone(),
            address,
            connector,
            broker_settings,
        )
        .await
        .unwrap();

        let port = _prepare_broker.address().port();
        client
            .advertise_port(prepare_keychain.keycard().identity(), port)
            .await;
        client
            .publish_card(prepare_keychain.keycard().clone(), Some(3))
            .await
            .unwrap();

        info!("Syncing with other brokers...");

        client
            .publish_card(KeyChain::random().keycard().clone(), Some(1))
            .await
            .unwrap();

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
                        return FullBrokerError::Fail.fail();
                    }
                },
            }
        };

        info!("Synced with other brokers. Making sure IdAssignments are published...");

        Ok(FullBroker {
            _signup_broker,
            _prepare_broker,
        })
    }
}
