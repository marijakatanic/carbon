use crate::{
    database::Database,
    discovery::Client,
    processing::{
        messages::CommitRequest,
        processor::commit::{errors::ServeCommitError, handlers},
        Processor,
    },
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use std::sync::Arc;

use talk::{
    crypto::KeyChain,
    net::{Listener, Session, SessionListener},
    sync::{fuse::Fuse, voidable::Voidable},
};

impl Processor {
    pub(in crate::processing) async fn run_commit<L>(
        keychain: KeyChain,
        discovery: Arc<Client>,
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
            let discovery = discovery.clone();
            let view = view.clone();
            let database = database.clone();

            fuse.spawn(async move {
                let _ = Processor::serve_commit(keychain, discovery, view, database, session).await;
            });
        }
    }

    async fn serve_commit(
        keychain: KeyChain,
        discovery: Arc<Client>,
        view: View,
        database: Arc<Voidable<Database>>,
        mut session: Session,
    ) -> Result<(), Top<ServeCommitError>> {
        let request = session
            .receive::<CommitRequest>()
            .await
            .pot(ServeCommitError::ConnectionError, here!())?;

        match request {
            CommitRequest::Ping => handlers::ping(session).await,
            CommitRequest::Batch(payloads) => {
                handlers::batch(
                    &keychain,
                    discovery.as_ref(),
                    &view,
                    database.as_ref(),
                    session,
                    payloads,
                )
                .await
            }
            CommitRequest::Completion(completion) => {
                handlers::completion(discovery.as_ref(), database.as_ref(), session, completion)
                    .await
            }
            _ => ServeCommitError::UnexpectedRequest.fail().spot(here!()),
        }
    }
}
