use crate::{
    database::Database,
    discovery::Client,
    processing::{
        messages::PrepareRequest,
        processor::prepare::{errors::ServePrepareError, handlers},
        Processor,
    },
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};
use log::{error, info};

use std::{sync::Arc, time::Instant};

use talk::{
    crypto::KeyChain,
    net::{Listener, Session, SessionListener},
    sync::{fuse::Fuse, voidable::Voidable},
};

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
                if let Err(e) =
                    Processor::serve_prepare(keychain, discovery, view, database, session).await
                {
                    error!("Error serving prepare: {:?}", e);
                }
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
                info!("Serving prepare!");

                let start = Instant::now();
                let result = handlers::batch(
                    &keychain,
                    discovery.as_ref(),
                    &view,
                    database.as_ref(),
                    session,
                    prepares,
                )
                .await;
                info!(
                    "Processed prepare batch in {} ms",
                    start.elapsed().as_millis()
                );
                info!("Finished serving prepare!");

                result
            }
            PrepareRequest::Commit(commit) => {
                info!("Serving commit!");

                let start = Instant::now();
                let commit =
                    handlers::commit(discovery.as_ref(), database.as_ref(), session, commit).await;
                info!(
                    "Processed prepare commit in {} ms",
                    start.elapsed().as_millis()
                );

                info!("Finished serving commit!");

                commit
            }
            _ => ServePrepareError::UnexpectedRequest.fail().spot(here!()),
        }
    }
}
