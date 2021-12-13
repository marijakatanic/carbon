use crate::{
    brokers::commit::{Broker, BrokerFailure, Brokerage, Submission, UnzippedBrokerages},
    commit::CompletionProof,
    data::PingBoard,
    processing::messages::CommitRequest,
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use futures::stream::{FuturesUnordered, StreamExt};

use std::sync::Arc;

use talk::{crypto::Identity, net::SessionConnector};

use zebra::vector::Vector;

#[derive(Doom)]
enum PublishError {
    #[doom(description("Connection failed"))]
    ConnectionFailed,
    #[doom(description("Connection error"))]
    ConnectionError,
}

impl Broker {
    pub(in crate::brokers::commit::broker) async fn broker(
        view: View,
        ping_board: PingBoard,
        connector: Arc<SessionConnector>,
        brokerages: Vec<Brokerage>,
    ) {
        // Unzip `brokerages` into its components

        let UnzippedBrokerages {
            payloads,
            commit_proofs,
            dependencies,
            completion_inlets,
        } = Brokerage::unzip(brokerages);

        let payloads = Vector::new(payloads).unwrap();
        let submission = Submission::new(payloads.clone(), commit_proofs, dependencies);

        // Orchestrate submission to obtain `BatchCompletion`

        let batch_completion =
            Broker::orchestrate(view.clone(), ping_board, connector.clone(), submission)
                .await
                .map_err(|_| BrokerFailure::Error);

        // Dispatch appropriate `CompletionProof` to all `serve` tasks

        for (index, completion_inlet) in completion_inlets.into_iter().enumerate() {
            let completion_proof = batch_completion.clone().map(|batch_completion| {
                let inclusion = payloads.prove(index);
                CompletionProof::new(batch_completion, inclusion)
            });

            let _ = completion_inlet.send(completion_proof);
        }

        // If `completion` is `Ok`, publish `BatchCommit` to all replicas

        if let Ok(batch_completion) = batch_completion {
            let request = CommitRequest::Completion(batch_completion);

            view.members()
                .keys()
                .copied()
                .map(|replica| Broker::publish(connector.as_ref(), &request, replica))
                .collect::<FuturesUnordered<_>>()
                .collect::<Vec<_>>()
                .await;
        }
    }

    async fn publish(
        connector: &SessionConnector,
        request: &CommitRequest,
        replica: Identity,
    ) -> Result<(), Top<PublishError>> {
        let mut session = connector
            .connect(replica)
            .await
            .pot(PublishError::ConnectionFailed, here!())?;

        session
            .send(request)
            .await
            .pot(PublishError::ConnectionError, here!())?;

        session.end();

        Ok(())
    }
}
