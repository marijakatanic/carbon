use crate::{
    database::Database,
    processing::{
        messages::SignupRequest,processor_settings::SignupSettings,
        processor::signup::{errors::ServeSignupError, message_handlers},
        Processor,
    },
    view::View,
};

use doomstack::{here, ResultExt, Top};

use std::sync::Arc;

use talk::{
    crypto::KeyChain,
    net::{Listener, Session, SessionListener},
    sync::{fuse::Fuse, voidable::Voidable},
};

impl Processor {
    pub(in crate::processing) async fn run_signup<L>(
        keychain: KeyChain,
        view: View,
        database: Arc<Voidable<Database>>,
        listener: L,
        settings: SignupSettings
    ) where
        L: Listener,
    {
        let mut listener = SessionListener::new(listener);
        let fuse = Fuse::new();

        loop {
            let (_, session) = listener.accept().await;

            let keychain = keychain.clone();
            let view = view.clone();
            let database = database.clone();
            let settings = settings.clone();

            fuse.spawn(async move {
                let _ = Processor::serve_signup(keychain, view, database, session, settings).await;
            });
        }
    }

    async fn serve_signup(
        keychain: KeyChain,
        view: View,
        database: Arc<Voidable<Database>>,
        mut session: Session,
        settings: SignupSettings
    ) -> Result<(), Top<ServeSignupError>> {
        let request = session
            .receive::<SignupRequest>()
            .await
            .pot(ServeSignupError::ConnectionError, here!())?;

        let response = {
            let mut database = database
                .lock()
                .pot(ServeSignupError::DatabaseVoid, here!())?;

            match request {
                SignupRequest::IdRequests(requests) => {
                    message_handlers::id_requests(&keychain, &view, &mut database, requests, &settings)?
                }

                SignupRequest::IdClaims(claims) => {
                    message_handlers::id_claims(&keychain, &view, &mut database, claims)?
                }
            }
        };

        session
            .send(&response)
            .await
            .pot(ServeSignupError::ConnectionError, here!())?;

        session.end();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{processing::test::System, signup::IdRequest};

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
        let request = IdRequest::new(&client, &view, allocator);

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
        let request = IdRequest::new(&client, &view, allocator);

        let mut assignments = brokers[0].signup(vec![request.clone()]).await;
        assert_eq!(assignments.len(), 1);

        let assignment = assignments.remove(0).unwrap();
        assignment.validate(&discovery_client).unwrap();
    }
}
