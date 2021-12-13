use crate::{
    brokers::commit::{Broker, BrokerFailure, Brokerage, Submission, UnzippedBrokerages},
    commit::CompletionProof,
    data::PingBoard,
    view::View,
};

use talk::net::SessionConnector;
use zebra::vector::Vector;

use std::sync::Arc;

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

        let batch_completion = Broker::orchestrate(view, ping_board, connector, submission)
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
    }
}
