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

use log::{error, info};

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
    view: View,
    root: Hash,
    aggregator: Aggregator<WitnessStatement>,
    errors: usize,
}

struct CommitCollector {
    view: View,
    root: Hash,
    shards: Vec<(KeyCard, BatchCommitShard)>,
    errors: usize,
}

#[derive(Doom)]
pub(in crate::brokers::prepare) enum OrchestrateError {
    #[doom(description("Failed to collect batch witness"))]
    WitnessCollectionFailed,
    #[doom(description("Failed to collect `BatchCommit`"))]
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
    #[doom(description("Invalid `BatchCommitShard`"))]
    InvalidCommitShard,
    #[doom(description("`Command` channel closed (most likely, the `Broker` is shutting down)"))]
    CommandChannelClosed,
}

#[derive(Doom)]
enum CollectorError {
    #[doom(description("Reached plurality of errors"))]
    ErrorPlurality,
}

impl Broker {
    pub(in crate::brokers::prepare) async fn orchestrate(
        discovery: Arc<Client>,
        view: View,
        ping_board: PingBoard,
        connector: Arc<SessionConnector>,
        submission: Submission,
        settings: BrokerTaskSettings,
    ) -> Result<BatchCommit, Top<OrchestrateError>> {
        // Submit a `submit` slave for each replica in `view`

        let submission = Arc::new(submission);

        let (update_inlet, mut update_outlet) = mpsc::unbounded_channel();
        let mut command_inlets = HashMap::new();

        let fuse = Fuse::new();

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

        // Obtain `PingBoard` rankings

        let rankings = ping_board.rankings();

        // Optimistically direct the fastest plurality of slaves to submit `submission`'s signatures

        for replica in &rankings[0..view.plurality()] {
            let _ = command_inlets
                .get_mut(replica)
                .unwrap()
                .send(Command::SubmitSignatures);
        }

        // Initialize `WitnessCollector`

        let mut witness_collector = WitnessCollector::new(view.clone(), submission.root());

        // Wait (or timeout) for the fastest plurality of slaves to produce witness shards

        info!("Waiting for witness shards...");

        let _ = time::timeout(
            settings.optimistic_witness_timeout,
            witness_collector.progress(&mut update_outlet),
        )
        .await;

        // If the fastest plurality of slaves failed to produce witness shards,
        // extend signature sumbission to fastest quorum of slaves

        // If `witness_collector.complete()` is `Err`, then a plurality of slaves
        // failed already, and collecting a `BatchCommit` is impossible
        let complete = witness_collector
            .complete()
            .pot(OrchestrateError::WitnessCollectionFailed, here!())?;

        if !complete {
            error!("Witness not complete!");

            for replica in &rankings[view.plurality()..view.quorum()] {
                let _ = command_inlets
                    .get_mut(replica)
                    .unwrap()
                    .send(Command::SubmitSignatures);
            }

            // Because a quorum of replicas is (theoretically) guaranteed to provide
            // a plurality of responses, collection of witness shards from a quorum
            // must carry on, without timeout, until success or failure.
            witness_collector.progress(&mut update_outlet).await;

            // Because `witness_collector.progress()` returned, if `witness_collector.complete()`
            // is `Ok`, then a plurality of witness shards was achieved.
            witness_collector
                .complete()
                .pot(OrchestrateError::WitnessCollectionFailed, here!())?;
        }

        // Finalize `witness_collector` to obtain witness

        let (commit_collector, witness) = witness_collector.finalize();

        info!("Obtained full witness");

        // Direct all slaves to send `witness`

        for command_inlet in command_inlets.values_mut() {
            let _ = command_inlet.send(Command::SubmitWitness(witness.clone()));
        }

        // Collect `BatchCommit` from a quorum of slaves

        let commit = commit_collector
            .run(&mut update_outlet)
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
        // In order to catch all `Err`s while maintaining `?`-syntax, all
        // operations are executed within the scope of an `async` block
        let result: Result<BatchCommitShard, Top<SubmitError>> = async {
            // Connect to `replica`

            let mut session = connector
                .connect(replica.identity())
                .await
                .pot(SubmitError::ConnectionFailed, here!())?;

            // Submit `Prepare`s (with a `PrepareRequest::Batch` request)

            session
                .send(&submission.requests.batch())
                .await
                .pot(SubmitError::ConnectionError, here!())?;

            // Wait for a `Command` from `orchestrate` master

            let command = command_outlet
                .recv()
                .await
                .ok_or(SubmitError::CommandChannelClosed.into_top())
                .spot(here!())?;

            // Obtain a witness: either directly from master; or by submitting signatures,
            // obtaining a witness shard, then trading the shard with master

            let witness = match command {
                // If `command` is `SubmitSignatures` then: submit signatures; receive a witness shard;
                // send the witness shard to master; receive a witness from master
                Command::SubmitSignatures => {
                    // Submit signatures (with a `PrepareRequest::Signatures` request)

                    session
                        .send(&submission.requests.signatures())
                        .await
                        .pot(SubmitError::ConnectionError, here!())?;

                    // Obtain a witness shard (if requested to do so, first provide `replica` with the
                    // `IdAssignment`s it is missing)

                    let response = session
                        .receive::<PrepareResponse>()
                        .await
                        .pot(SubmitError::ConnectionError, here!())?;

                    let shard = match response {
                        // If `response` is `UnknownIds`, then `replica` misses some `IdAssignment`s,
                        // required to validate the submitted signatures
                        PrepareResponse::UnknownIds(unknown_ids) => {
                            error!("Replica has unknown ids!");

                            // Gather the necessary `IdAssignments`. Assignments are requested
                            // by `Id`, prompting a binary search on `submission.assignments()`
                            // (which was sorted by `Id` by `Broker::prepare`)
                            let id_assignments = unknown_ids
                                .into_iter()
                                .map(|id| {
                                    // If `id` is not present in `submission.assignments()`, then
                                    // `replica` is Byzantine
                                    let index = submission
                                        .assignments()
                                        .binary_search_by_key(&id, |assignment| assignment.id())
                                        .map_err(|_| SubmitError::MalformedResponse.into_top())
                                        .spot(here!())?;

                                    Ok(submission.assignments()[index].clone())
                                })
                                .collect::<Result<Vec<IdAssignment>, Top<SubmitError>>>()?;

                            // Send missing `IdAssignments`

                            session
                                .send(&PrepareRequest::Assignments(id_assignments))
                                .await
                                .pot(SubmitError::ConnectionError, here!())?;

                            // Receive witness shard (a correct `replica` cannot provide any
                            // response other than `WitnessShard`)

                            let response = session
                                .receive::<PrepareResponse>()
                                .await
                                .pot(SubmitError::ConnectionError, here!())?;

                            match response {
                                PrepareResponse::WitnessShard(shard) => Ok(shard),
                                _ => SubmitError::UnexpectedResponse.fail().spot(here!()),
                            }
                        }

                        // If `command` is `WitnessShard`, return witness shard
                        PrepareResponse::WitnessShard(shard) => Ok(shard),

                        _ => SubmitError::UnexpectedResponse.fail().spot(here!()),
                    }?;

                    // Verify `shard`

                    let statement = WitnessStatement::new(submission.root());

                    shard
                        .verify([&replica], &statement)
                        .pot(SubmitError::InvalidWitnessShard, here!())?;

                    // Send `shard` to master

                    let _ = update_inlet.send((replica.identity(), Update::WitnessShard(shard)));

                    // Receive witness from master, return witness

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

                // If `command` is `SubmitWitness`, return witness
                Command::SubmitWitness(witness) => witness,
            };

            // Send `witness` (with a `PrepareRequest::Witness` request)

            session
                .send(&PrepareRequest::Witness(witness))
                .await
                .pot(SubmitError::ConnectionError, here!())?;

            // Receive `BatchCommitShard` (a correct `replica` cannot provide
            // any response other than `CommitShard`)

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

        // If `shard` is `Ok`, send `BatchCommitShard` to master, otherwise signal `Error`

        let _ = match result {
            Ok(shard) => update_inlet.send((replica.identity(), Update::CommitShard(shard))),
            Err(_) => update_inlet.send((replica.identity(), Update::Error)),
        };
    }
}

impl WitnessCollector {
    pub fn new(view: View, root: Hash) -> Self {
        let statement = WitnessStatement::new(root);
        let aggregator = Aggregator::new(view.clone(), statement);

        WitnessCollector {
            view,
            root,
            aggregator,
            errors: 0,
        }
    }

    fn succeeded(&self) -> bool {
        self.aggregator.multiplicity() >= self.view.plurality()
    }

    fn failed(&self) -> bool {
        self.errors >= self.view.plurality()
    }

    async fn progress(&mut self, update_outlet: &mut UpdateOutlet) {
        while !self.succeeded() && !self.failed() {
            // A copy of `update_inlet` is held by `orchestrate`.
            // As a result, `update_outlet.recv()` cannot return `None`.
            match update_outlet.recv().await.unwrap() {
                (replica, Update::WitnessShard(shard)) => {
                    let keycard = self.view.members().get(&replica).unwrap();
                    self.aggregator.add_unchecked(keycard, shard);
                }
                (_, Update::Error) => {
                    self.errors += 1;
                }
                _ => {
                    panic!("`WitnessCollector::progress` received an unexpected `Update`");
                }
            }
        }
    }

    pub fn complete(&self) -> Result<bool, Top<CollectorError>> {
        if self.failed() {
            CollectorError::ErrorPlurality.fail().spot(here!())
        } else {
            Ok(self.succeeded())
        }
    }

    pub fn finalize(self) -> (CommitCollector, Certificate) {
        let commit_collector = CommitCollector::new(self.view, self.root, self.errors);
        let (_, witness) = self.aggregator.finalize();

        (commit_collector, witness)
    }
}

impl CommitCollector {
    fn new(view: View, root: Hash, errors: usize) -> Self {
        CommitCollector {
            view,
            root,
            shards: Vec::new(),
            errors,
        }
    }

    fn succeeded(&self) -> bool {
        self.shards.len() >= self.view.quorum()
    }

    fn failed(&self) -> bool {
        self.errors >= self.view.plurality()
    }

    async fn run(
        mut self,
        update_outlet: &mut UpdateOutlet,
    ) -> Result<BatchCommit, Top<CollectorError>> {
        while !self.succeeded() && !self.failed() {
            // A copy of `update_inlet` is held by `orchestrate`.
            // As a result, `update_outlet.recv()` cannot return `None`.
            match update_outlet.recv().await.unwrap() {
                (replica, Update::CommitShard(shard)) => {
                    let keycard = self.view.members().get(&replica).unwrap().clone();
                    self.shards.push((keycard, shard));
                }
                (_, Update::Error) => {
                    self.errors += 1;
                }
                (_, Update::WitnessShard(_)) => {}
            }
        }

        if self.succeeded() {
            Ok(BatchCommit::new(self.view, self.root, self.shards))
        } else {
            CollectorError::ErrorPlurality.fail().spot(here!())
        }
    }
}
