use crate::{
    brokers::prepare::{Broker, Submission},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use std::{collections::HashMap, sync::Arc};

use talk::{crypto::Identity, net::SessionConnector, sync::fuse::Fuse};

use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

type CommandInlet = UnboundedSender<Command>;
type CommandOutlet = UnboundedReceiver<Command>;

type UpdateInlet = UnboundedSender<(Identity, Update)>;
type UpdateOutlet = UnboundedReceiver<(Identity, Update)>;

enum Command {}

enum Update {
    Success,
    Error,
}

#[derive(Doom)]
enum SubmitError {
    #[doom(description("Connection failed"))]
    ConnectionFailed,
    #[doom(description("Connection error"))]
    ConnectionError,
}

impl Broker {
    pub(in crate::brokers::prepare::broker) async fn orchestrate(
        view: View,
        connector: Arc<SessionConnector>,
        submission: Submission,
    ) {
        let submission = Arc::new(submission);

        let (update_inlet, update_outlet) = mpsc::unbounded_channel();

        let fuse = Fuse::new();

        let mut command_inlets = HashMap::new();

        for replica in view.members().keys().copied() {
            let connector = connector.clone();
            let submission = submission.clone();
            let update_inlet = update_inlet.clone();

            let (command_inlet, command_outlet) = mpsc::unbounded_channel();
            command_inlets.insert(replica, command_inlet);

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
        let result: Result<(), Top<SubmitError>> = async {
            let mut session = connector
                .connect(replica)
                .await
                .pot(SubmitError::ConnectionFailed, here!())?;

            session
                .send(&submission.requests.batch)
                .await
                .pot(SubmitError::ConnectionError, here!())?;

            Ok(())
        }
        .await;

        let _ = match result {
            Ok(_) => update_inlet.send((replica, Update::Success)),
            Err(_) => update_inlet.send((replica, Update::Error)),
        };
    }
}
