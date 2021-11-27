use crate::{crypto::Identify, processing::{messages::{SignupRequest, SignupResponse}, processor_settings::SignupSettings}, signup::{IdAllocation, IdAssignment, IdAssignmentAggregator, IdClaim, IdRequest}, view::View};

use futures::stream::{FuturesUnordered, StreamExt};

use talk::{
    crypto::{primitives::multi::Signature as MultiSignature, Identity, KeyChain},
    link::context::ConnectDispatcher,
    net::{test::TestConnector, SessionConnector},
};

pub(crate) struct TestBroker {
    keychain: KeyChain,
    view: View,
    signup_connector: SessionConnector,
}

impl TestBroker {
    pub fn new(keychain: KeyChain, view: View, connector: TestConnector) -> TestBroker {
        let dispatcher = ConnectDispatcher::new(connector);

        let signup_context = format!("{:?}::processor::signup", view.identifier());
        let signup_connector = SessionConnector::new(dispatcher.register(signup_context));

        Self {
            keychain,
            view,
            signup_connector,
        }
    }

    pub async fn id_requests(&self, requests: Vec<IdRequest>) -> Vec<IdAllocation> {
        assert!(requests.len() > 0);

        assert!(requests
            .iter()
            .all(|request| request.view() == self.view.identifier()));

        let allocator = requests[0].allocator();

        assert!(self.view.members().contains_key(&allocator));

        assert!(requests
            .iter()
            .all(|request| request.allocator() == allocator));

        for request in requests.iter() {
            request.validate(SignupSettings::default().work_difficulty).unwrap();
        }

        let mut session = self.signup_connector.connect(allocator).await.unwrap();

        session
            .send(&SignupRequest::IdRequests(requests))
            .await
            .unwrap();

        let response = session.receive().await.unwrap();
        session.end();

        match response {
            SignupResponse::IdAllocations(allocations) => allocations,
            _ => panic!("unexpected response"),
        }
    }

    pub async fn id_claims(
        &self,
        assigner: Identity,
        claims: Vec<IdClaim>,
    ) -> Vec<Result<MultiSignature, IdClaim>> {
        assert!(claims.len() > 0);

        assert!(claims
            .iter()
            .all(|claim| claim.view() == self.view.identifier()));

        let allocator = claims[0].allocator();

        assert!(self.view.members().contains_key(&allocator));

        assert!(claims.iter().all(|claim| claim.allocator() == allocator));

        for claim in claims.iter() {
            claim.validate(SignupSettings::default().work_difficulty).unwrap();
        }

        let mut session = self.signup_connector.connect(assigner).await.unwrap();

        session
            .send(&SignupRequest::IdClaims(claims.clone()))
            .await
            .unwrap();

        let response = session.receive().await.unwrap();
        session.end();

        match response {
            SignupResponse::IdAssignments(assignments) => assignments,
            _ => panic!("unexpected response"),
        }
    }

    pub async fn signup(&self, requests: Vec<IdRequest>) -> Vec<Option<IdAssignment>> {
        let allocations = self.id_requests(requests.clone()).await;

        let claims = requests
            .into_iter()
            .zip(allocations)
            .map(|(request, allocation)| {
                allocation.validate(&request).unwrap();
                IdClaim::new(request, allocation)
            })
            .collect::<Vec<_>>();

        let mut unordered = self
            .view
            .members()
            .keys()
            .map(|assigner_identity| {
                let assigner_keycard = self.view.members().get(assigner_identity).cloned().unwrap();
                let claims = claims.clone();

                async move {
                    (
                        assigner_keycard,
                        self.id_claims(*assigner_identity, claims).await,
                    )
                }
            })
            .collect::<FuturesUnordered<_>>();

        let mut aggregators = claims
            .iter()
            .map(|claim| {
                Some(IdAssignmentAggregator::new(
                    self.view.clone(),
                    claim.id(),
                    claim.client(),
                ))
            })
            .collect::<Vec<_>>();

        for _ in 0..self.view.quorum() {
            let (assigner, assignments) = unordered.next().await.unwrap();

            if assignments.len() != claims.len() {
                panic!("unexpected number of assignments")
            }

            let progress = aggregators
                .iter_mut()
                .zip(assignments)
                .filter(|(aggregator, _)| aggregator.is_some());

            for (aggregator, assignment) in progress {
                match assignment {
                    Ok(signature) => {
                        aggregator
                            .as_mut()
                            .unwrap()
                            .add(&assigner, signature)
                            .unwrap();
                    }
                    Err(collision) => {
                        let id = aggregator.as_ref().unwrap().id();
                        let client = aggregator.as_ref().unwrap().keycard();

                        collision.validate(SignupSettings::default().work_difficulty).unwrap();
                        assert_eq!(collision.id(), id);
                        assert_eq!(collision.client(), client);

                        aggregator.take();
                    }
                }
            }
        }

        aggregators
            .into_iter()
            .map(|aggregator| aggregator.map(|aggregator| aggregator.finalize()))
            .collect::<Vec<_>>()
    }
}
