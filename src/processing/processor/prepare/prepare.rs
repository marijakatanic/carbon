use crate::{
    database::Database,
    discovery::Client,
    processing::{
        messages::PrepareRequest, processor::prepare::errors::ServePrepareError, Processor,
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

use super::handlers;

impl Processor {
    pub(in crate::processing) async fn run_prepare<L>(
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
                let _ =
                    Processor::serve_prepare(keychain, discovery, view, database, session).await;
            });
        }
    }

    async fn serve_prepare(
        keychain: KeyChain,
        discovery: Arc<Client>,
        view: View,
        database: Arc<Voidable<Database>>,
        mut session: Session,
    ) -> Result<(), Top<ServePrepareError>> {
        let request = session
            .receive::<PrepareRequest>()
            .await
            .pot(ServePrepareError::ConnectionError, here!())?;

        match request {
            PrepareRequest::Ping => handlers::ping(session).await,
            PrepareRequest::Batch(prepares) => {
                handlers::batch(
                    &keychain,
                    discovery.as_ref(),
                    &view,
                    database.as_ref(),
                    session,
                    prepares,
                )
                .await
            }
            PrepareRequest::Commit(commit) => {
                handlers::commit(discovery.as_ref(), database.as_ref(), session, commit).await
            }
            _ => ServePrepareError::UnexpectedRequest.fail().spot(here!()),
        }
    }
}
