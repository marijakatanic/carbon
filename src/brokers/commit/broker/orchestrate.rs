use crate::{
    brokers::commit::{submission::Submission, Broker},
    commit::{BatchCompletion, BatchCompletionAggregator, BatchCompletionShard, WitnessStatement},
    crypto::{Aggregator, Certificate},
    data::PingBoard,
    discovery::Client,
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use std::{collections::HashMap, sync::Arc, time::Duration};

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
    SubmitWitnessRequest,
    SubmitWitness(Certificate),
}

enum Update {
    WitnessShard(MultiSignature),
    CompletionShard(BatchCompletionShard),
    Error,
}

struct WitnessCollector {
    view: View,
    root: Hash,
    aggregator: Aggregator<WitnessStatement>,
    errors: usize,
}

struct CompletionCollector {
    view: View,
    root: Hash,
    aggregator: BatchCompletionAggregator,
    errors: usize,
}

#[derive(Doom)]
pub(in crate::brokers::commit::broker) enum OrchestrateError {
    #[doom(description("Failed to collect batch witness"))]
    WitnessCollectionFailed,
    #[doom(description("Failed to collect `BatchCompletion`"))]
    CompletionCollectionFailed,
}

#[derive(Doom)]
enum SubmitError {
    #[doom(description("Connection failed"))]
    ConnectionFailed,
    #[doom(description("Connection error"))]
    ConnectionError,
}

#[derive(Doom)]
enum CollectorError {
    #[doom(description("Reached plurality of errors"))]
    ErrorPlurality,
}

impl Broker {
    pub(in crate::brokers::commit::broker) async fn orchestrate(
        discovery: Arc<Client>,
        view: View,
        ping_board: PingBoard,
        connector: Arc<SessionConnector>,
        submission: Submission,
    ) -> Result<BatchCompletion, Top<OrchestrateError>> {
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
                .send(Command::SubmitWitnessRequest);
        }

        // Initialize `WitnessCollector`

        let mut witness_collector = WitnessCollector::new(view.clone(), submission.root());

        // Wait (or timeout) for the fastest plurality of slaves to produce witness shards

        let _ = time::timeout(
            Duration::from_secs(1), // TODO: Add settings
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
            for replica in &rankings[view.plurality()..view.quorum()] {
                let _ = command_inlets
                    .get_mut(replica)
                    .unwrap()
                    .send(Command::SubmitWitnessRequest);
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

        // Direct all slaves to send `witness`

        for command_inlet in command_inlets.values_mut() {
            let _ = command_inlet.send(Command::SubmitWitness(witness.clone()));
        }

        // Collect `BatchCommit` from a quorum of slaves

        let commit = commit_collector
            .run(&mut update_outlet)
            .await
            .pot(OrchestrateError::CompletionCollectionFailed, here!())?;

        Ok(commit)
    }

    async fn submit(
        _discovery: Arc<Client>,
        _view: View,
        _connector: Arc<SessionConnector>,
        _replica: KeyCard,
        _submission: Arc<Submission>,
        _command_outlet: CommandOutlet,
        _update_inlet: UpdateInlet,
    ) {
        todo!()
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
                    self.aggregator.add(keycard, shard).unwrap();
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

    pub fn finalize(self) -> (CompletionCollector, Certificate) {
        let completion_collector = CompletionCollector::new(self.view, self.root, self.errors);
        let (_, witness) = self.aggregator.finalize();

        (completion_collector, witness)
    }
}

impl CompletionCollector {
    fn new(view: View, root: Hash, errors: usize) -> Self {
        let aggregator = BatchCompletionAggregator::new(view.clone(), root);

        CompletionCollector {
            view,
            root,
            aggregator,
            errors,
        }
    }

    fn succeeded(&self) -> bool {
        self.aggregator.complete()
    }

    fn failed(&self) -> bool {
        self.errors >= self.view.plurality()
    }

    async fn run(
        mut self,
        update_outlet: &mut UpdateOutlet,
    ) -> Result<BatchCompletion, Top<CollectorError>> {
        while !self.succeeded() && !self.failed() {
            // A copy of `update_inlet` is held by `orchestrate`.
            // As a result, `update_outlet.recv()` cannot return `None`.
            match update_outlet.recv().await.unwrap() {
                (replica, Update::CompletionShard(shard)) => {
                    let keycard = self.view.members().get(&replica).unwrap().clone();
                    self.aggregator.add(&keycard, shard);
                }
                (_, Update::Error) => {
                    self.errors += 1;
                }
                (_, Update::WitnessShard(_)) => {}
            }
        }

        if self.succeeded() {
            Ok(self.aggregator.finalize())
        } else {
            CollectorError::ErrorPlurality.fail().spot(here!())
        }
    }
}
