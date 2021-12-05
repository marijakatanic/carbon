use crate::{
    brokers::prepare::{Broker, Submission},
    view::View,
};

use std::sync::Arc;

use talk::net::SessionConnector;

impl Broker {
    pub(in crate::brokers::prepare::broker) async fn orchestrate(
        view: View,
        connector: Arc<SessionConnector>,
        submission: Submission,
    ) {
    }
}
