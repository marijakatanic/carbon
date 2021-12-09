use crate::{
    database::Database,
    discovery::Client,
    processing::{
        messages::SignupRequest,
        processor::signup::{errors::ServeSignupError, handlers},
        processor_settings::Signup,
        Processor,
    },
    view::View,
};

use doomstack::{here, ResultExt, Top};
use log::{error, info};
use tokio::sync::Semaphore;

use std::{sync::Arc, time::Instant};

use talk::{
    crypto::KeyChain,
    net::{Listener, Session, SessionListener},
    sync::{fuse::Fuse, voidable::Voidable},
};

impl Processor {
    pub(in crate::processing) async fn run_signup<L>(
        keychain: KeyChain,
        discovery: Arc<Client>,
        view: View,
        database: Arc<Voidable<Database>>,
        listener: L,
        settings: Signup,
    ) where
        L: Listener,
    {
        let mut listener = SessionListener::new(listener);
        let fuse = Fuse::new();

        let semaphore = Arc::new(Semaphore::new(1));

        loop {
            let (_, session) = listener.accept().await;

            let keychain = keychain.clone();
            let discovery = discovery.clone();
            let view = view.clone();
            let database = database.clone();
            let settings = settings.clone();
            let semaphore = semaphore.clone();

            fuse.spawn(async move {
                if let Err(e) =
                    Processor::serve_signup(keychain, discovery, view, database, session, semaphore, settings)
                        .await
                {
                    error!("Error serving signup: {:?}", e);
                }
            });
        }
    }

    async fn serve_signup(
        keychain: KeyChain,
        discovery: Arc<Client>,
        view: View,
        database: Arc<Voidable<Database>>,
        mut session: Session,
        semaphore: Arc<Semaphore>,
        settings: Signup,
    ) -> Result<(), Top<ServeSignupError>> {
        let request = session
            .receive::<SignupRequest>()
            .await
            .pot(ServeSignupError::ConnectionError, here!())?;

        

        let response = {
            let _permit = semaphore.acquire().await.unwrap();

            match request {
                SignupRequest::IdRequests(requests) => {
                    info!("Received id requests");
                    let start = Instant::now();
                    let answer = handlers::id_requests(&keychain, &view, database.as_ref(), requests, &settings)?;
                    info!("Processed id requests in {} ms.", start.elapsed().as_millis());
                    answer
                }

                SignupRequest::IdClaims(claims) => {
                    info!("Received id claims");
                    let start = Instant::now();
                    let answer = handlers::id_claims(&keychain, &view, database.as_ref(), claims, &settings)?;
                    info!("Processed id claims in {} ms.", start.elapsed().as_millis());
                    answer
                }

                SignupRequest::IdAssignments(assignments) => {
                    info!("Received id assignments");
                    let start = Instant::now();
                    let answer = handlers::id_assignments(discovery.as_ref(), database.as_ref(), assignments)?;
                    info!("Processed id assignments in {} ms.", start.elapsed().as_millis());
                    answer
                }
            }
        };

        info!("Sending response");

        session
            .send(&response)
            .await
            .pot(ServeSignupError::ConnectionError, here!())?;

        info!("Response sent");

        session.end();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        processing::test::System,
        signup::{IdRequest, SignupSettings},
    };

    #[tokio::test]
    async fn allocation_priority() {
        let System {
            view,
            brokers,
            processors,
            ..
        } = System::setup(4, 1).await;

        let allocator = processors[0].0.keycard().identity();

        let client = KeyChain::random();
        let request = IdRequest::new(
            &client,
            &view,
            allocator,
            SignupSettings::default().work_difficulty,
        );

        let mut allocations = brokers[0].id_requests(vec![request.clone()]).await;
        assert_eq!(allocations.len(), 1);

        let allocation = allocations.remove(0);
        allocation.validate(&request).unwrap();
        assert!(allocation.id() <= u32::MAX as u64);
    }

    #[tokio::test]
    async fn signup() {
        let System {
            view,
            discovery_server: _discovery_server,
            discovery_client,
            brokers,
            processors,
        } = System::setup(4, 1).await;

        let allocator = processors[0].0.keycard().identity();

        let client = KeyChain::random();
        let request = IdRequest::new(
            &client,
            &view,
            allocator,
            SignupSettings::default().work_difficulty,
        );

        let mut assignments = brokers[0].signup(vec![request.clone()]).await;
        assert_eq!(assignments.len(), 1);

        let assignment = assignments.remove(0).unwrap();
        assignment.validate(&discovery_client).unwrap();
    }
}
