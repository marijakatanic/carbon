use crate::{
    crypto::Identify,
    processing::{SignupRequest, SignupResponse},
    signup::IdRequest,
    view::View,
};

use talk::crypto::KeyChain;
use talk::link::context::ConnectDispatcher;
use talk::net::test::TestConnector;
use talk::net::Connector;

pub(crate) struct TestBroker {
    keychain: KeyChain,
    view: View,
    dispatcher: ConnectDispatcher,
}

impl TestBroker {
    pub fn new(keychain: KeyChain, view: View, connector: TestConnector) -> TestBroker {
        let dispatcher = ConnectDispatcher::new(connector);

        Self {
            keychain,
            view,
            dispatcher,
        }
    }

    pub async fn id_requests(&self, id_requests: Vec<IdRequest>) -> SignupResponse {
        let signup_context = format!("{:?}::processor::signup", self.view.identifier());
        let broker_connector = self.dispatcher.register(signup_context);

        assert!(id_requests.len() > 0);
        assert!(id_requests
            .iter()
            .all(|id_request| id_request.view() == self.view.identifier()));

        let assigner = id_requests[0].assigner();

        assert!(self.view.members().contains_key(&assigner));
        assert!(id_requests
            .iter()
            .all(|id_request| id_request.assigner() == assigner));

        let mut connection = broker_connector.connect(assigner).await.unwrap();

        connection
            .send(&SignupRequest::IdRequests(id_requests))
            .await
            .unwrap();

        connection.receive::<SignupResponse>().await.unwrap()
    }
}
