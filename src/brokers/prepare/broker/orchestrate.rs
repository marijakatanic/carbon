use crate::{
    brokers::prepare::{broker_settings::BrokerTaskSettings, Broker, Submission},
    crypto::{Aggregator, Certificate},
    data::PingBoard,
    discovery::Client,
    prepare::{BatchCommit, BatchCommitShard, WitnessStatement},
    processing::messages::{PrepareRequest, PrepareResponse},
    signup::IdAssignment,
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use std::{collections::HashMap, sync::Arc};

use talk::{
    crypto::{
        primitives::{hash::Hash, multi::Signature as MultiSignature},
        Identity, KeyCard,
    },
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

struct WitnessCollector {
    aggregator: Aggregator<WitnessStatement>,
    errors: usize,
}

struct CommitCollector {
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

#[derive(Doom)]
enum ProgressError {
    #[doom(description("Error overflow"))]
    ErrorOverflow,
}

impl Broker {
    pub(in crate::brokers::prepare::broker) async fn orchestrate(
        discovery: Arc<Client>,
        view: View,
        connector: Arc<SessionConnector>,
        ping_board: PingBoard,
        submission: Submission,
        settings: BrokerTaskSettings,
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

        let mut witness_collector = WitnessCollector::new(view.clone(), submission.root());

        // Wait for plurality to respond, with a timeout

        let _ = time::timeout(
            settings.optimistic_witness_timeout,
            witness_collector.progress(&view, &mut update_outlet),
        )
        .await;

        let complete = witness_collector
            .complete(&view)
            .pot(OrchestrateError::WitnessCollectionFailed, here!())?;

        if !complete {
            for replica in &rankings[view.plurality()..view.quorum()] {
                let _ = command_inlets
                    .get_mut(replica)
                    .unwrap()
                    .send(Command::SubmitSignatures);
            }

            witness_collector.progress(&view, &mut update_outlet).await;

            witness_collector
                .complete(&view)
                .pot(OrchestrateError::WitnessCollectionFailed, here!())?;
        }

        let (commit_collector, witness) = witness_collector.finalize();

        // Send witness to all replicas

        for command_inlet in command_inlets.values_mut() {
            let _ = command_inlet.send(Command::SubmitWitness(witness.clone()));
        }

        // Wait for a quorum of commit shards

        let commit = commit_collector
            .run(view, submission.root(), &mut update_outlet)
            .await
            .pot(OrchestrateError::CommitCollectionFailed, here!())?;

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
                .send(&submission.requests.batch())
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
                        .send(&submission.requests.signatures())
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
                                        .assignments()
                                        .binary_search_by_key(&id, |assignment| assignment.id())
                                        .map_err(|_| SubmitError::MalformedResponse.into_top())
                                        .spot(here!())?;

                                    Ok(submission.assignments()[index].clone())
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

                    let statement = WitnessStatement::new(submission.root());

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
                    submission.root(),
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

impl WitnessCollector {
    fn new(view: View, root: Hash) -> Self {
        let statement = WitnessStatement::new(root);

        WitnessCollector {
            aggregator: Aggregator::new(view, statement),
            errors: 0,
        }
    }

    async fn progress(&mut self, view: &View, update_outlet: &mut UpdateOutlet) {
        while self.aggregator.multiplicity() < view.plurality() && self.errors < view.plurality() {
            // A copy of `update_inlet` is held by `orchestrate`.
            // As a result, `update_outlet.recv()` cannot return `None`.
            match update_outlet.recv().await.unwrap() {
                (replica, Update::WitnessShard(shard)) => {
                    let keycard = view.members().get(&replica).unwrap();
                    self.aggregator.add(keycard, shard).unwrap();
                }
                (_, Update::Error) => {
                    self.errors += 1;
                }
                _ => {
                    panic!("unexpected `Update`");
                }
            }
        }
    }

    pub fn complete(&self, view: &View) -> Result<bool, Top<ProgressError>> {
        if self.errors < view.plurality() {
            Ok(self.aggregator.multiplicity() >= view.plurality())
        } else {
            ProgressError::ErrorOverflow.fail().spot(here!())
        }
    }

    pub fn finalize(self) -> (CommitCollector, Certificate) {
        let commit_collector = CommitCollector::with_errors(self.errors);
        let (_, witness) = self.aggregator.finalize();

        (commit_collector, witness)
    }
}

impl CommitCollector {
    fn with_errors(errors: usize) -> Self {
        CommitCollector {
            shards: Vec::new(),
            errors,
        }
    }

    async fn run(
        mut self,
        view: View,
        root: Hash,
        update_outlet: &mut UpdateOutlet,
    ) -> Result<BatchCommit, Top<ProgressError>> {
        while self.shards.len() < view.quorum() && self.errors < view.plurality() {
            // A copy of `update_inlet` is held by `orchestrate`.
            // As a result, `update_outlet.recv()` cannot return `None`.
            match update_outlet.recv().await.unwrap() {
                (replica, Update::CommitShard(shard)) => {
                    let keycard = view.members().get(&replica).unwrap().clone();
                    self.shards.push((keycard, shard));
                }
                (_, Update::Error) => {
                    self.errors += 1;
                }
                (_, Update::WitnessShard(_)) => {}
            }
        }

        if self.shards.len() >= view.quorum() {
            Ok(BatchCommit::new(view, root, self.shards))
        } else {
            ProgressError::ErrorOverflow.fail().spot(here!())
        }
    }
}
