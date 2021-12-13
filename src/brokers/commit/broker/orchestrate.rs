use crate::{
    brokers::commit::{submission::Submission, Broker},
    commit::BatchCompletion,
    data::PingBoard,
    discovery::Client,
    view::View,
};

use doomstack::{Doom, Top};

use std::sync::Arc;

use talk::net::SessionConnector;

#[derive(Doom)]
pub(in crate::brokers::commit::broker) enum OrchestrateError {
    #[doom(description("Failed to collect batch witness"))]
    WitnessCollectionFailed,
    #[doom(description("Failed to collect `BatchCompletion`"))]
    CompletionCollectionFailed,
}

impl Broker {
    pub(in crate::brokers::commit::broker) async fn orchestrate(
        _discovery: Arc<Client>,
        _view: View,
        _ping_board: PingBoard,
        _connector: Arc<SessionConnector>,
        _submission: Submission,
    ) -> Result<BatchCompletion, Top<OrchestrateError>> {
        todo!()
    }
}
