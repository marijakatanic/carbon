use crate::{
    brokers::{
        prepare::{BrokerFailure, Inclusion, Request as PrepareRequest},
        signup::BrokerFailure as SignupBrokerFailure,
    },
    external::parameters::{BrokerParameters, Export, Parameters},
    prepare::{BatchCommit, Prepare, ReductionStatement},
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
        num_clients: usize,
    ) -> Result<Self, Top<ClientError>> {
        // Load default parameters if none are specified.
        let BrokerParameters {
            signup_batch_number,
            signup_batch_size,
            prepare_batch_number,
            prepare_batch_size,
            prepare_single_sign_percentage,
            ..
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

        let mut addresses = Vec::new();
        for broker in shard.iter() {
            addresses.push(client.get_address(broker.identity()).await.unwrap());
        }

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
            ..prepare_batch_size / num_clients)
            .map(|_| {
                let keychain = KeyChain::random();
                let request = IdRequest::new(&keychain, &genesis, allocator.clone(), 0);

                (keychain, request)
            })
            .unzip();

        info!("Getting assignments...");

        let assignments: Vec<IdAssignment> = batch_requests
            .into_iter()
            .enumerate()
            .map(|(num, id_request)| {
                let address = addresses[num / 100].clone();

                async move {
                    let stream = TcpStream::connect(address.clone()).await.unwrap();
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

        let prepare_request_batches = (0..1)
            .map(|height| prepare(height as u64, &batch_key_chains, &assignments))
            .collect::<Vec<_>>();

        time::sleep(Duration::from_secs(10)).await;

        info!("Sending operations...");
        let prepare_shard = get_shard(&client, 3).await?;
        let broker = prepare_shard[0].clone();
        let address = client.get_address(broker.identity()).await.unwrap();

        let reduction_shard = batch_key_chains[0]
            .multisign(&ReductionStatement::new(hash::hash(&0).unwrap()))
            .unwrap();

        info!("Getting assignments...");
        for (height, batch) in prepare_request_batches.into_iter().enumerate() {
            let _commits: Vec<BatchCommit> = batch_key_chains
                .iter()
                .zip(batch.into_iter())
                .enumerate()
                .map(|(i, (keychain, prepare_request))| {
                    let address = address.clone();

                    async move {
                        if i == 0 {
                            info!("Client sending prepare for height {}", height);
                        }

                        let stream = TcpStream::connect(address).await.unwrap();
                        let mut connection: PlainConnection = stream.into();

                        connection.send(&prepare_request).await.unwrap();

                        let inclusion = connection
                            .receive::<Result<Inclusion, BrokerFailure>>()
                            .await
                            .unwrap()
                            .unwrap();

                        // When benchmarking, we only simulate the processing time of a single client
                        // In real life, each client is separate and only processes their own transaction
                        // so other clients' processing time should not be included in latency
                        if i == 0 {
                            let _ = inclusion
                                .certify_reduction(&keychain, prepare_request.prepare())
                                .unwrap();
                        }

                        connection.send(&reduction_shard).await.unwrap();

                        let result = connection
                            .receive::<Result<BatchCommit, BrokerFailure>>()
                            .await
                            .unwrap()
                            .unwrap();

                        if i == 0 {
                            info!("Client finished prepare for height {}", height);
                        }

                        result
                    }
                })
                .collect::<FuturesUnordered<_>>()
                .collect::<Vec<_>>()
                .await;
        }

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
) -> Vec<PrepareRequest> {
    let commitment = hash::hash(&0).unwrap();
    let fake_prepare = Prepare::new(id_assignments[0].id(), height, commitment.clone());
    let fake_signature = clients[0].sign(&fake_prepare).unwrap();

    id_assignments
        .iter()
        .cloned()
        .map(|assignment| {
            let prepare = Prepare::new(assignment.id(), height, commitment.clone());
            PrepareRequest {
                assignment,
                prepare,
                signature: fake_signature.clone(),
            }
        })
        .collect()
}
