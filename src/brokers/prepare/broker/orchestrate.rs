use crate::{
    brokers::prepare::{Broker, Submission},
    crypto::{Aggregator, Certificate},
    data::PingBoard,
    discovery::Client,
    prepare::{BatchCommit, BatchCommitShard, WitnessStatement},
    processing::messages::{PrepareRequest, PrepareResponse},
    signup::IdAssignment,
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use std::{collections::HashMap, sync::Arc, time::Duration};

use talk::{
    crypto::{primitives::multi::Signature as MultiSignature, Identity, KeyCard},
    net::SessionConnector,
    sync::fuse::Fuse,
};

use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    time,
};

type CommandInlet = UnboundedSender<Command>;
type CommandOutlet = UnboundedReceiver<Command>;

type UpdateInlet = UnboundedSender<(Identity, Update)>;
type UpdateOutlet = UnboundedReceiver<(Identity, Update)>;

enum Command {
    SubmitSignatures,
    SubmitWitness(Certificate),
}

enum Update {
    WitnessShard(MultiSignature),
    CommitShard(BatchCommitShard),
    Error,
}

struct WitnessProgress {
    aggregator: Aggregator<WitnessStatement>,
    errors: usize,
}

struct CommitProgress {
    shards: Vec<(KeyCard, BatchCommitShard)>,
    errors: usize,
}

#[derive(Doom)]
pub(in crate::brokers::prepare::broker) enum OrchestrateError {
    #[doom(description("Failed to collect batch witness"))]
    WitnessCollectionFailed,
    #[doom(description("Failed to collect batch commit"))]
    CommitCollectionFailed,
}

#[derive(Doom)]
enum SubmitError {
    #[doom(description("Connection failed"))]
    ConnectionFailed,
    #[doom(description("Connection error"))]
    ConnectionError,
    #[doom(description("Unexpected response"))]
    UnexpectedResponse,
    #[doom(description("Malformed response"))]
    MalformedResponse,
    #[doom(description("Invalid witness shard"))]
    InvalidWitnessShard,
    #[doom(description("Invalid commit shard"))]
    InvalidCommitShard,
    #[doom(description("Command channel closed"))]
    CommandChannelClosed,
}

impl Broker {
    pub(in crate::brokers::prepare::broker) async fn orchestrate(
        discovery: Arc<Client>,
        view: View,
        connector: Arc<SessionConnector>,
        ping_board: PingBoard,
        submission: Submission,
        fast_witness_timeout: Duration,
    ) -> Result<BatchCommit, Top<OrchestrateError>> {
        let submission = Arc::new(submission);

        let (update_inlet, mut update_outlet) = mpsc::unbounded_channel();

        let fuse = Fuse::new();

        let mut command_inlets = HashMap::new();

        for replica in view.members().values().cloned() {
            let discovery = discovery.clone();
            let view = view.clone();
            let connector = connector.clone();
            let submission = submission.clone();
            let update_inlet = update_inlet.clone();

            let (command_inlet, command_outlet) = mpsc::unbounded_channel();
            command_inlets.insert(replica.identity(), command_inlet);

            fuse.spawn(async move {
                let _ = Broker::submit(
                    discovery,
                    view,
                    connector,
                    replica,
                    submission,
                    command_outlet,
                    update_inlet,
                )
                .await;
            });
        }

        let rankings = ping_board.rankings();

        // Instruct fastest plurality to submit signatures

        for replica in &rankings[0..view.plurality()] {
            let _ = command_inlets
                .get_mut(replica)
                .unwrap()
                .send(Command::SubmitSignatures);
        }

        let statement = WitnessStatement::new(submission.root);
        let aggregator = Aggregator::new(view.clone(), statement);

        let mut progress = WitnessProgress {
            aggregator,
            errors: 0,
        };

        // Wait for plurality to respond, with a timeout

        let _ = time::timeout(
            fast_witness_timeout,
            witness_progress(&view, &mut update_outlet, &mut progress),
        )
        .await;

        if progress.errors >= view.plurality() {
            return OrchestrateError::WitnessCollectionFailed
                .fail()
                .spot(here!());
        }

        // If not enough responded in time, extend request to quorum, wait for shards without timeout

        if progress.aggregator.multiplicity() < view.plurality() {
            for replica in &rankings[view.plurality()..view.quorum()] {
                let _ = command_inlets
                    .get_mut(replica)
                    .unwrap()
                    .send(Command::SubmitSignatures);
            }

            witness_progress(&view, &mut update_outlet, &mut progress).await;
        }

        if progress.errors >= view.plurality() {
            return OrchestrateError::WitnessCollectionFailed
                .fail()
                .spot(here!());
        }

        let (_, witness) = progress.aggregator.finalize();

        // Send witness to all replicas

        for command_inlet in command_inlets.values_mut() {
            let _ = command_inlet.send(Command::SubmitWitness(witness.clone()));
        }

        // Wait for a quorum of commit shards

        let mut progress = CommitProgress {
            shards: Vec::new(),
            errors: progress.errors,
        };

        commit_progress(&view, &mut update_outlet, &mut progress).await;

        if progress.errors >= view.plurality() {
            return OrchestrateError::CommitCollectionFailed
                .fail()
                .spot(here!());
        }

        let commit = BatchCommit::new(view, submission.root, progress.shards);

        Ok(commit)
    }

    async fn submit(
        discovery: Arc<Client>,
        view: View,
        connector: Arc<SessionConnector>,
        replica: KeyCard,
        submission: Arc<Submission>,
        mut command_outlet: CommandOutlet,
        update_inlet: UpdateInlet,
    ) {
        let result: Result<BatchCommitShard, Top<SubmitError>> = async {
            let mut session = connector
                .connect(replica.identity())
                .await
                .pot(SubmitError::ConnectionFailed, here!())?;

            session
                .send(&submission.requests.batch)
                .await
                .pot(SubmitError::ConnectionError, here!())?;

            let command = command_outlet
                .recv()
                .await
                .ok_or(SubmitError::CommandChannelClosed.into_top())
                .spot(here!())?;

            let witness = match command {
                Command::SubmitSignatures => {
                    session
                        .send(&submission.requests.signatures)
                        .await
                        .pot(SubmitError::ConnectionError, here!())?;

                    let response = session
                        .receive::<PrepareResponse>()
                        .await
                        .pot(SubmitError::ConnectionError, here!())?;

                    let shard = match response {
                        PrepareResponse::UnknownIds(unknown_ids) => {
                            let id_assignments = unknown_ids
                                .into_iter()
                                .map(|id| {
                                    let index = submission
                                        .assignments
                                        .binary_search_by_key(&id, |assignment| assignment.id())
                                        .map_err(|_| SubmitError::MalformedResponse.into_top())
                                        .spot(here!())?;

                                    Ok(submission.assignments[index].clone())
                                })
                                .collect::<Result<Vec<IdAssignment>, Top<SubmitError>>>()?;

                            session
                                .send(&PrepareRequest::Assignments(id_assignments))
                                .await
                                .pot(SubmitError::ConnectionError, here!())?;

                            let response = session
                                .receive::<PrepareResponse>()
                                .await
                                .pot(SubmitError::ConnectionError, here!())?;

                            match response {
                                PrepareResponse::WitnessShard(shard) => Ok(shard),
                                _ => SubmitError::UnexpectedResponse.fail().spot(here!()),
                            }
                        }
                        PrepareResponse::WitnessShard(shard) => Ok(shard),
                        _ => SubmitError::UnexpectedResponse.fail().spot(here!()),
                    }?;

                    let statement = WitnessStatement::new(submission.root);

                    shard
                        .verify([&replica], &statement)
                        .pot(SubmitError::InvalidWitnessShard, here!())?;

                    let _ = update_inlet.send((replica.identity(), Update::WitnessShard(shard)));

                    let command = command_outlet
                        .recv()
                        .await
                        .ok_or(SubmitError::CommandChannelClosed.into_top())
                        .spot(here!())?;

                    match command {
                        Command::SubmitWitness(witness) => witness,
                        _ => {
                            panic!("unexpected `Command`");
                        }
                    }
                }
                Command::SubmitWitness(witness) => witness,
            };

            session
                .send(&PrepareRequest::Witness(witness))
                .await
                .pot(SubmitError::ConnectionError, here!())?;

            let response = session
                .receive::<PrepareResponse>()
                .await
                .pot(SubmitError::ConnectionError, here!())?;

            let shard = match response {
                PrepareResponse::CommitShard(shard) => Ok(shard),
                _ => SubmitError::UnexpectedResponse.fail().spot(here!()),
            }?;

            shard
                .validate(
                    discovery.as_ref(),
                    &view,
                    submission.root,
                    submission.prepares(),
                    &replica,
                )
                .pot(SubmitError::InvalidCommitShard, here!())?;

            Ok(shard)
        }
        .await;

        let _ = match result {
            Ok(shard) => update_inlet.send((replica.identity(), Update::CommitShard(shard))),
            Err(_) => update_inlet.send((replica.identity(), Update::Error)),
        };
    }
}

async fn witness_progress(
    view: &View,
    update_outlet: &mut UpdateOutlet,
    progress: &mut WitnessProgress,
) {
    while progress.aggregator.multiplicity() < view.plurality()
        && progress.errors < view.plurality()
    {
        match update_outlet.recv().await.unwrap() {
            // TODO: Check the `unwrap` above
            (replica, Update::WitnessShard(shard)) => {
                let keycard = view.members().get(&replica).unwrap();
                progress.aggregator.add(&keycard, shard).unwrap();
            }
            (_, Update::Error) => {
                progress.errors += 1;
            }
            _ => {
                panic!("unexpected `Update`");
            }
        }
    }
}

async fn commit_progress(
    view: &View,
    update_outlet: &mut UpdateOutlet,
    progress: &mut CommitProgress,
) {
    while progress.shards.len() < view.quorum() && progress.errors < view.plurality() {
        match update_outlet.recv().await.unwrap() {
            // TODO: Check the `unwrap` above
            (replica, Update::CommitShard(shard)) => {
                let keycard = view.members().get(&replica).unwrap().clone();
                progress.shards.push((keycard, shard));
            }
            (_, Update::Error) => {
                progress.errors += 1;
            }
            (_, Update::WitnessShard(_)) => {}
        }
    }
}
