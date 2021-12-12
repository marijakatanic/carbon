use crate::{
    brokers::commit::{Broker, Brokerage},
    data::PingBoard,
    discovery::Client,
    view::View,
};

use talk::net::SessionConnector;

use std::sync::Arc;

impl Broker {
    pub(in crate::brokers::commit::broker) async fn broker(
        _discovery: Arc<Client>,
        _view: View,
        _ping_board: PingBoard,
        _connector: Arc<SessionConnector>,
        _brokerages: Vec<Brokerage>,
    ) {
    }
}
