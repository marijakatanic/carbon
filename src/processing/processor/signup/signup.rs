use crate::{
    database::Database,
    processing::{
        messages::SignupRequest,
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

            fuse.spawn(async move {
                let _ = Processor::serve_signup(keychain, view, database, session).await;
            });
        }
    }

    async fn serve_signup(
        keychain: KeyChain,
        view: View,
        database: Arc<Voidable<Database>>,
        mut session: Session,
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
                    message_handlers::id_requests(&keychain, &view, &mut database, requests)?
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
