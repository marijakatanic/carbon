use crate::{
    brokers::commit::{Broker, Brokerage, Submission, UnzippedBrokerages},
    data::PingBoard,
    discovery::Client,
    view::View,
};

use talk::net::SessionConnector;
use zebra::vector::Vector;

use std::sync::Arc;

impl Broker {
    pub(in crate::brokers::commit::broker) async fn broker(
        _discovery: Arc<Client>,
        _view: View,
        _ping_board: PingBoard,
        _connector: Arc<SessionConnector>,
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

        let _submission = Arc::new(Submission::new(
            payloads.clone(),
            commit_proofs,
            dependencies,
        ));
    }
}
