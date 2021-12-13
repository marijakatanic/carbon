use crate::{
    brokers::commit::{submission::Submission, Broker},
    commit::{BatchCompletion, BatchCompletionShard},
    crypto::Certificate,
    data::PingBoard,
    discovery::Client,
    view::View,
};

use doomstack::{Doom, Top};

use std::{collections::HashMap, sync::Arc};

use talk::{
    crypto::{primitives::multi::Signature as MultiSignature, Identity, KeyCard},
    net::SessionConnector,
    sync::fuse::Fuse,
};

use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

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

        let (update_inlet, _update_outlet) = mpsc::unbounded_channel();
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

        todo!()
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
