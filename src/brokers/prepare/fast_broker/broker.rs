use crate::{
    brokers::prepare::{
        broker_settings::BrokerTaskSettings, Broker, BrokerFailure, FastBroker, Submission,
    },
    data::PingBoard,
    discovery::Client,
    prepare::BatchCommit,
    view::View,
};

use std::sync::Arc;

use log::error;
use talk::net::SessionConnector;

impl FastBroker {
    pub(in crate::brokers::prepare::fast_broker) async fn broker(
        discovery: Arc<Client>,
        view: View,
        ping_board: PingBoard,
        connector: Arc<SessionConnector>,
        submission: Submission,
        settings: BrokerTaskSettings,
    ) -> Result<BatchCommit, BrokerFailure> {
        // Orchestrate submission of `submission`
        match Broker::orchestrate(discovery, view, ping_board, connector, submission, settings)
            .await
        {
            Err(e) => {
                error!("Orchestrate failed. {:?}", e);
                Err(BrokerFailure::Error)
            }
            Ok(c) => Ok(c),
        }
    }
}
