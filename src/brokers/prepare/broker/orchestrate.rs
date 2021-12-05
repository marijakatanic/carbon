use crate::{
    brokers::prepare::{ping_board::PingBoard, Broker, Submission},
    prepare::WitnessStatement,
    processing::messages::PrepareResponse,
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

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
    SubmitSignatures,
}

enum Update {
    WitnessShard(MultiSignature),
    Success,
    Error,
}

#[derive(Doom)]
enum SubmitError {
    #[doom(description("Connection failed"))]
    ConnectionFailed,
    #[doom(description("Connection error"))]
    ConnectionError,
    #[doom(description("Unexpected response"))]
    UnexpectedResponse,
    #[doom(description("Invalid witness shard"))]
    InvalidWitnessShard,
    #[doom(description("Command channel closed"))]
    CommandChannelClosed,
}

impl Broker {
    pub(in crate::brokers::prepare::broker) async fn orchestrate(
        view: View,
        connector: Arc<SessionConnector>,
        ping_board: PingBoard,
        submission: Submission,
    ) {
        let submission = Arc::new(submission);

        let (update_inlet, update_outlet) = mpsc::unbounded_channel();

        let fuse = Fuse::new();

        let mut command_inlets = HashMap::new();

        for replica in view.members().values().cloned() {
            let connector = connector.clone();
            let submission = submission.clone();
            let update_inlet = update_inlet.clone();

            let (command_inlet, command_outlet) = mpsc::unbounded_channel();
            command_inlets.insert(replica.identity(), command_inlet);

            fuse.spawn(async move {
                Broker::submit(connector, replica, submission, command_outlet, update_inlet)
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
    }

    async fn submit(
        connector: Arc<SessionConnector>,
        replica: KeyCard,
        submission: Arc<Submission>,
        mut command_outlet: CommandOutlet,
        update_inlet: UpdateInlet,
    ) {
        let result: Result<(), Top<SubmitError>> = async {
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

            match command {
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
                        PrepareResponse::WitnessShard(shard) => Ok(shard),
                        _ => SubmitError::UnexpectedResponse.fail().spot(here!()),
                    }?;

                    let statement = WitnessStatement::new(submission.root);

                    shard
                        .verify([&replica], &statement)
                        .pot(SubmitError::InvalidWitnessShard, here!())?;

                    let _ = update_inlet.send((replica.identity(), Update::WitnessShard(shard)));
                }
            }

            Ok(())
        }
        .await;

        let _ = match result {
            Ok(_) => update_inlet.send((replica.identity(), Update::Success)),
            Err(_) => update_inlet.send((replica.identity(), Update::Error)),
        };
    }
}
