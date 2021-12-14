use crate::{
    account::{Entry, Operation},
    brokers::{
        commit::Request,
        prepare::{BrokerFailure, Inclusion, Request as PrepareRequest},
        signup::BrokerFailure as SignupBrokerFailure,
    },
    commit::{Commit, CommitProof, Completion, CompletionProof, Payload},
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
                let address = addresses[100 * num / (prepare_batch_size / num_clients)].clone();

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

        info!("All IdAssignments obtained.");

        let prepare_request_batches = (0..10)
            .map(|height| prepare(height as u64, &batch_key_chains, &assignments))
            .collect::<Vec<_>>();

        time::sleep(Duration::from_secs(10)).await;

        info!("Getting prepare shard...");
        let prepare_shard = get_shard(&client, 3).await?;

        info!("Getting commit shard");
        let commit_shard = get_shard(&client, 4).await?;

        let mut prepare_addresses = Vec::new();
        for broker in prepare_shard.iter() {
            prepare_addresses.push(client.get_address(broker.identity()).await.unwrap());
        }

        let mut commit_addresses = Vec::new();
        for broker in commit_shard.iter() {
            commit_addresses.push(client.get_address(broker.identity()).await.unwrap());
        }

        let reduction_shard = batch_key_chains[0]
            .multisign(&ReductionStatement::new(hash::hash(&0).unwrap()))
            .unwrap();

        client
            .publish_card(KeyChain::random().keycard(), Some(1))
            .await
            .unwrap();
        let _ = get_shard(&client, 1).await?;

        info!("Awaiting to be in the middle of the throughput...");
        time::sleep(Duration::from_secs(20 + 5)).await;

        info!("Starting latency test...");
        for (height, batch) in prepare_request_batches.into_iter().enumerate() {
            let _completions: Vec<Completion> = batch_key_chains
                .iter()
                .zip(batch.into_iter())
                .enumerate()
                .map(|(num, (keychain, prepare_request))| {
                    let prepare_address =
                        prepare_addresses[100 * num / (prepare_batch_size / num_clients)].clone();
                    let commit_address =
                        commit_addresses[100 * num / (prepare_batch_size / num_clients)].clone();

                    async move {
                        if num == 0 {
                            info!("Client sending prepare for height {}", height);
                        }

                        let stream = TcpStream::connect(prepare_address).await.unwrap();
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
                        if num == 0 {
                            let _ = inclusion
                                .certify_reduction(&keychain, prepare_request.prepare())
                                .unwrap();
                        }

                        connection.send(&reduction_shard).await.unwrap();

                        let batch_commit = connection
                            .receive::<Result<BatchCommit, BrokerFailure>>()
                            .await
                            .unwrap()
                            .unwrap();

                        let commit_proof = CommitProof::new(batch_commit, inclusion.proof);

                        let payload = Payload::new(
                            Entry {
                                id: prepare_request.prepare().id(),
                                height: (prepare_request.prepare().height()),
                            },
                            Operation::withdraw(
                                prepare_request.prepare().id(),
                                prepare_request.prepare().height() - 1,
                                0,
                            ),
                        );

                        let commit = Commit::new(commit_proof, payload.clone());

                        // Commit

                        let request = Request::new(commit, None);

                        let stream = TcpStream::connect(commit_address).await.unwrap();
                        let mut connection: PlainConnection = stream.into();

                        connection.send(&request).await.unwrap();

                        let completion_proof = match connection
                            .receive::<Result<CompletionProof, BrokerFailure>>()
                            .await
                            .unwrap()
                        {
                            Ok(proof) => proof,
                            Err(e) => {
                                error!("Completion error! {:?}", e);
                                Err(e).unwrap()
                            }
                        };

                        let withdrawal = Completion::new(completion_proof, payload);

                        if num == 0 {
                            info!("Client finished prepare for height {}", height);
                        }

                        withdrawal
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
    let fake_prepare = Prepare::new(
        Entry {
            id: id_assignments[0].id(),
            height,
        },
        commitment.clone(),
    );
    let fake_signature = clients[0].sign(&fake_prepare).unwrap();

    id_assignments
        .iter()
        .cloned()
        .map(|assignment| {
            let payload = Payload::new(
                Entry {
                    id: assignment.id(),
                    height: (height + 1),
                },
                Operation::withdraw(assignment.id(), height, 0),
            );

            let prepare = payload.prepare();

            PrepareRequest {
                assignment,
                prepare,
                signature: fake_signature.clone(),
            }
        })
        .collect()
}

// Copy paste code

// {
//     let payload = Payload::new(
//         Entry {
//             id: assignment.id(),
//             height: 1,
//         },
//         Operation::withdraw(assignment.id(), 0, 0),
//     );

//     let prepare = payload.prepare();

//     // Prepare

//     let request = PrepareRequest::new(
//         &client_keychain,
//         assignment.clone(),
//         prepare.height(),
//         prepare.commitment(),
//     );

//     let stream = TcpStream::connect(prepare_broker.address()).await.unwrap();
//     let mut connection: PlainConnection = stream.into();

//     connection.send(&request).await.unwrap();

//     let inclusion = connection
//         .receive::<Result<PrepareInclusion, PrepareBrokerFailure>>()
//         .await
//         .unwrap()
//         .unwrap();

//     let reduction_shard = inclusion
//         .certify_reduction(&client_keychain, request.prepare())
//         .unwrap();

//     connection.send(&reduction_shard).await.unwrap();

//     let batch_commit = connection
//         .receive::<Result<BatchCommit, PrepareBrokerFailure>>()
//         .await
//         .unwrap()
//         .unwrap();

//     let commit_proof = CommitProof::new(batch_commit, inclusion.proof);

//     let commit = Commit::new(commit_proof, payload.clone());

//     println!("[Withdraw] Prepare completed.");

//     // Commit

//     let request = Request::new(commit, None);

//     let stream = TcpStream::connect(commit_broker.address()).await.unwrap();
//     let mut connection: PlainConnection = stream.into();

//     connection.send(&request).await.unwrap();

//     let completion_proof = connection
//         .receive::<Result<CompletionProof, BrokerFailure>>()
//         .await
//         .unwrap()
//         .unwrap();

//     let withdrawal = Completion::new(completion_proof, payload);

//     println!("[Withdraw] Commit completed.");
//     }
