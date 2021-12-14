use crate::{
    brokers::{prepare::Request, signup::BrokerFailure as SignupBrokerFailure},
    external::parameters::{BrokerParameters, Export, Parameters},
    prepare::Prepare,
    signup::{IdAssignment, IdRequest},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use futures::stream::{FuturesUnordered, StreamExt};
use log::{error, info};
use tokio::{net::TcpStream, time};

use std::time::Duration;

use talk::{
    crypto::{primitives::hash, KeyCard, KeyChain},
    link::rendezvous::{Client as RendezvousClient, ClientError as RendezvousClientError, ShardId},
    net::{traits::TcpConnect, PlainConnection},
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
        // Load default parameters if none are specified.
        let BrokerParameters {
            signup_batch_number,
            signup_batch_size,
            prepare_batch_number,
            prepare_batch_size,
            prepare_single_sign_percentage,
        } = match parameters_file {
            Some(filename) => {
                Parameters::read(filename)
                    .pot(ClientError::Fail, here!())?
                    .broker
            }
            None => Parameters::default().broker,
        };

        info!("Signup batch number: {}", signup_batch_number);
        info!("Signup batch size: {}", signup_batch_size);
        info!("Prepare batch number: {}", prepare_batch_number);
        info!("Prepare batch size: {}", prepare_batch_size);
        info!(
            "Prepare single sign percentage: {}",
            prepare_single_sign_percentage
        );

        info!("Getting broker keycard");

        let client = RendezvousClient::new(rendezvous.clone(), Default::default());
        let shard = get_shard(&client, 2).await?;

        info!(
            "Obtained shard! Honest broker identities {:?}",
            shard
                .iter()
                .map(|keycard| keycard.identity())
                .collect::<Vec<_>>()
        );

        let broker = shard[0].clone();
        let address = client.get_address(broker.identity()).await.unwrap();

        let mut shard = get_shard(&client, 0).await?;
        shard.sort_by_key(|keycard| keycard.identity());

        info!(
            "Obtained shard! Replica identities {:?}",
            shard
                .iter()
                .map(|keycard| keycard.identity())
                .collect::<Vec<_>>()
        );

        let allocator = shard.iter().next().unwrap().identity();
        let genesis = View::genesis(shard);

        let (batch_key_chains, batch_requests): (Vec<KeyChain>, Vec<IdRequest>) = (0
            ..signup_batch_size)
            .map(|_| {
                let keychain = KeyChain::random();
                let request = IdRequest::new(&keychain, &genesis, allocator.clone(), 0);

                (keychain, request)
            })
            .unzip();

        info!("Getting assignments...");
        let assignments: Vec<IdAssignment> = batch_requests
            .into_iter()
            .map(|id_request| {
                let address = address.clone();
                async move {
                    let stream = TcpStream::connect(address).await.unwrap();
                    let mut connection: PlainConnection = stream.into();

                    connection.send(&id_request).await.unwrap();

                    connection
                        .receive::<Result<IdAssignment, SignupBrokerFailure>>()
                        .await
                        .unwrap()
                        .unwrap()
                }
            })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;

        info!("All alocations obtained.");

        let prepared_requests = (0..10)
            .map(|height| prepare(height as u64, &batch_key_chains, &assignments))
            .collect::<Vec<_>>();

        info!("Waiting to start prepare...");
        let _ = get_shard(&client, 1).await?;

        Ok(Client {})
    }
}

async fn get_shard(
    client: &RendezvousClient,
    number: ShardId,
) -> Result<Vec<KeyCard>, Top<ClientError>> {
    loop {
        match client.get_shard(number).await {
            Ok(shard) => break Ok(shard),
            Err(e) => match e.top() {
                RendezvousClientError::ShardIncomplete => {
                    info!("Shard still incomplete, sleeping...");
                    time::sleep(Duration::from_millis(500)).await
                }
                _ => {
                    error!("Error obtaining first shard view");
                    return ClientError::Fail.fail();
                }
            },
        }
    }
}

fn prepare(
    height: u64,
    clients: &Vec<KeyChain>,
    id_assignments: &Vec<IdAssignment>,
) -> Vec<Request> {
    let commitment = hash::hash(&0).unwrap();
    let fake_prepare = Prepare::new(id_assignments[0].id(), height, commitment.clone());
    let fake_signature = clients[0].sign(&fake_prepare).unwrap();

    id_assignments
        .iter()
        .cloned()
        .map(|assignment| {
            let prepare = Prepare::new(assignment.id(), height, commitment.clone());
            Request {
                assignment,
                prepare,
                signature: fake_signature.clone(),
            }
        })
        .collect()
}
