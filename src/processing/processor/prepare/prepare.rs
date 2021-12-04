use crate::{
    database::Database,
    discovery::Client,
    processing::{
        processor::prepare::{errors::ServePrepareError, steps},
        Processor,
    },
    view::View,
};

use doomstack::Top;

use std::sync::Arc;

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
        let batch = steps::witnessed_batch(
            &keychain,
            discovery.as_ref(),
            &view,
            database.as_ref(),
            &mut session,
        )
        .await?;

        let root = batch.root();
        let shard = steps::apply_batch(&keychain, &view, database.as_ref(), batch).await?;

        steps::trade_commits(
            discovery.as_ref(),
            database.as_ref(),
            &mut session,
            root,
            shard,
        )
        .await?;

        session.end();

        Ok(())
    }
}
