use crate::{
    brokers::commit::{Broker, BrokerFailure, Brokerage, Submission, UnzippedBrokerages},
    data::PingBoard,
    discovery::Client,
    view::View,
};

use talk::net::SessionConnector;
use zebra::vector::Vector;

use std::sync::Arc;

impl Broker {
    pub(in crate::brokers::commit::broker) async fn broker(
        discovery: Arc<Client>,
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
            completion_inlets: _completion_inlets,
        } = Brokerage::unzip(brokerages);

        let payloads = Vector::new(payloads).unwrap();
        let submission = Submission::new(payloads.clone(), commit_proofs, dependencies);

        let _completion = Broker::orchestrate(discovery, view, ping_board, connector, submission)
            .await
            .map_err(|_| BrokerFailure::Error);
    }
}
