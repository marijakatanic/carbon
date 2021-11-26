use crate::{
    crypto::Identify,
    processing::{SignupRequest, SignupResponse},
    signup::{IdAssignment, IdAssignmentAggregator, IdClaim, IdRequest},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use futures::stream::{FuturesUnordered, StreamExt};

use std::sync::Arc;

use talk::{
    crypto::{primitives::multi::Signature as MultiSignature, KeyCard, KeyChain},
    link::context::{ConnectDispatcher, Connector as ContextConnector},
    net::{test::TestConnector, Connector},
};

pub(crate) struct TestBroker {
    keychain: KeyChain,
    view: View,
    signup_connector: Arc<ContextConnector>,
}

#[derive(Doom)]
pub(crate) enum TestBrokerError {
    #[doom(description("Signup error"))]
    SignupError,
}

impl TestBroker {
    pub fn new(keychain: KeyChain, view: View, connector: TestConnector) -> TestBroker {
        let dispatcher = ConnectDispatcher::new(connector);

        let signup_context = format!("{:?}::processor::signup", view.identifier());
        let signup_connector = Arc::new(dispatcher.register(signup_context));

        Self {
            keychain,
            view,
            signup_connector,
        }
    }

    pub async fn id_requests(&self, id_requests: Vec<IdRequest>) -> SignupResponse {
        assert!(id_requests.len() > 0);
        assert!(id_requests
            .iter()
            .all(|id_request| id_request.view() == self.view.identifier()));

        let allocator = id_requests[0].allocator();

        assert!(self.view.members().contains_key(&allocator));
        assert!(id_requests
            .iter()
            .all(|id_request| id_request.allocator() == allocator));

        let mut connection = self.signup_connector.connect(allocator).await.unwrap();

        connection
            .send(&SignupRequest::IdRequests(id_requests))
            .await
            .unwrap();

        connection.receive::<SignupResponse>().await.unwrap()
    }

    pub async fn signup(
        &self,
        id_requests: Vec<IdRequest>,
    ) -> Result<Vec<Option<IdAssignment>>, Top<TestBrokerError>> {
        let response = self.id_requests(id_requests.clone()).await;

        let allocations = match response {
            SignupResponse::IdAllocations(allocations) => allocations,
            _ => panic!("unexpected response"),
        };

        let id_claims = id_requests
            .into_iter()
            .zip(allocations.into_iter())
            .map(|(request, allocation)| {
                allocation
                    .validate(&request)
                    .pot(TestBrokerError::SignupError, here!())?;

                Ok(IdClaim::new(request, allocation))
            })
            .collect::<Result<Vec<IdClaim>, Top<TestBrokerError>>>()?;

        let mut unordered = self
            .view
            .members()
            .values()
            .cloned()
            .map(|replica| {
                let connector = self.signup_connector.clone();
                let claims = id_claims.clone();

                TestBroker::process_claim(claims, replica, connector)
            })
            .collect::<FuturesUnordered<_>>();

        let mut aggregators = id_claims
            .iter()
            .map(|claim| {
                Some(IdAssignmentAggregator::new(
                    self.view.clone(),
                    claim.id(),
                    claim.client(),
                ))
            })
            .collect::<Vec<_>>();

        let mut count = 0;

        while count < self.view.quorum() {
            if let Some(result) = unordered.next().await {
                let (keycard, signatures) = result?;

                if signatures.len() != id_claims.len() {
                    continue; // Bad replica
                }

                if aggregators
                    .iter_mut()
                    .zip(signatures)
                    .filter(|(aggregator, _)| aggregator.is_some())
                    .all(|(aggregator, signature)| match signature {
                        Err(existing_claim) => {
                            let id = aggregator.as_ref().unwrap().id();
                            let client = aggregator.as_ref().unwrap().keycard();

                            if existing_claim.validate().is_ok()
                                && existing_claim.id() == id
                                && existing_claim.client() != client
                            {
                                aggregator.take();
                                true
                            } else {
                                false // Bad replica
                            }
                        }
                        Ok(signature) => aggregator
                            .as_mut()
                            .unwrap()
                            .add(&keycard, signature)
                            .is_ok(),
                    })
                {
                    count += 1;
                }
            }
        }

        Ok(aggregators
            .into_iter()
            .map(|aggregator| aggregator.map(|aggregator| aggregator.finalize()))
            .collect::<Vec<_>>())
    }

    async fn process_claim(
        claims: Vec<IdClaim>,
        replica: KeyCard,
        connector: Arc<ContextConnector>,
    ) -> Result<(KeyCard, Vec<Result<MultiSignature, IdClaim>>), Top<TestBrokerError>> {
        let mut connection = connector
            .connect(replica.identity())
            .await
            .pot(TestBrokerError::SignupError, here!())?;

        connection
            .send(&SignupRequest::IdClaims(claims.clone()))
            .await
            .pot(TestBrokerError::SignupError, here!())?;

        let response: SignupResponse = connection
            .receive()
            .await
            .pot(TestBrokerError::SignupError, here!())?;

        match response {
            SignupResponse::IdAssignments(assignments) => Ok((replica, assignments)),
            _ => TestBrokerError::SignupError.fail().spot(here!()),
        }
    }
}
