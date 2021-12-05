use crate::{
    brokers::prepare::{Broker, Submission},
    view::View,
};

use std::sync::Arc;

use talk::{crypto::Identity, net::SessionConnector, sync::fuse::Fuse};

use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

type CommandInlet = UnboundedSender<Command>;
type CommandOutlet = UnboundedReceiver<Command>;

type UpdateInlet = UnboundedSender<(Identity, Update)>;
type UpdateOutlet = UnboundedReceiver<(Identity, Update)>;

enum Command {}

enum Update {}

impl Broker {
    pub(in crate::brokers::prepare::broker) async fn orchestrate(
        view: View,
        connector: Arc<SessionConnector>,
        submission: Submission,
    ) {
        let submission = Arc::new(submission);

        let (update_inlet, update_outlet) = mpsc::unbounded_channel();

        let fuse = Fuse::new();

        for replica in view.members().keys().copied() {
            let connector = connector.clone();
            let submission = submission.clone();

            let (command_inlet, command_outlet) = mpsc::unbounded_channel();
            let update_inlet = update_inlet.clone();

            fuse.spawn(async move {
                Broker::submit(connector, replica, submission, command_outlet, update_inlet)
            });
        }
    }

    async fn submit(
        connector: Arc<SessionConnector>,
        replica: Identity,
        submission: Arc<Submission>,
        command_outlet: CommandOutlet,
        update_inlet: UpdateInlet,
    ) {
    }
}
