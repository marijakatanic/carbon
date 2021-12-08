use crate::{
    database::Database, discovery::Client as DiscoveryClient, processing::Processor, view::View,
};

use doomstack::{Doom, Top};

use log::{error, info};

use std::{sync::Arc, time::Duration};

use talk::{
    crypto::KeyChain,
    link::rendezvous::{Client as RendezvousClient, ClientError, Connector, Listener},
    net::traits::TcpConnect,
};

use tokio::time;

#[derive(Doom)]
pub enum ReplicaError {
    #[doom(description("Fail"))]
    Fail,
}

pub struct Replica {
    _processor: Processor,
}

impl Replica {
    pub async fn new<A: 'static + Clone + TcpConnect>(
        rendezvous: A,
        discovery: A,
    ) -> Result<Self, Top<ReplicaError>> {
        // Load default parameters if none are specified.
        // let parameters = match parameter_file {
        //     Some(filename) => Parameters::read(filename)?,
        //     None => Parameters::default(),
        // };

        let keychain = KeyChain::random();
        let keycard = keychain.keycard();

        info!("Identity {:?} generated", keycard.identity());

        info!("Creating listener...");

        let listener =
            Listener::new(rendezvous.clone(), keychain.clone(), Default::default()).await;

        info!("Creating connector...");

        let connector = Connector::new(rendezvous.clone(), keychain.clone(), Default::default());

        info!("Publishing KeyCard... {:?}", keycard);

        let client = RendezvousClient::new(rendezvous, Default::default());
        client.publish_card(keycard.clone(), Some(0)).await.unwrap();

        // Wait for everyone to register
        let shard = loop {
            match client.get_shard(0).await {
                Ok(shard) => break shard,
                Err(e) => match e.top() {
                    ClientError::ShardIncomplete => {
                        info!("Shard still incomplete, sleeping...");
                        time::sleep(Duration::from_millis(500)).await
                    }
                    _ => {
                        error!("Error obtaining first shard view");
                        return ReplicaError::Fail.fail();
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
        let discovery = Arc::new(DiscoveryClient::new(
            genesis.clone(),
            discovery,
            Default::default(),
        ));
        let database = Database::new();

        info!("Initializing processor...");

        let _processor = Processor::new(
            keychain,
            discovery,
            genesis,
            database,
            connector,
            listener,
            Default::default(),
        );

        info!("Processor initialized. Going to sleep...");

        info!("Woke");

        Ok(Self {_processor})
    }
}
