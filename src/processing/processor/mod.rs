use crate::{
    crypto::Identify, database::Database, discovery::Client, processing::ProcessorSettings,
    view::View,
};

use std::sync::Arc;

use talk::{
    crypto::KeyChain,
    link::context::{ConnectDispatcher, ListenDispatcher},
    net::{Connector, Listener},
    sync::{fuse::Fuse, voidable::Voidable},
};

pub(crate) struct Processor {
    database: Arc<Voidable<Database>>,
    _fuse: Fuse,
}

impl Processor {
    pub fn new<C, L>(
        keychain: KeyChain,
        discovery: Arc<Client>,
        view: View,
        database: Database,
        connector: C,
        listener: L,
        settings: ProcessorSettings,
    ) -> Self
    where
        C: Connector,
        L: Listener,
    {
        let database = Arc::new(Voidable::new(database));

        let _connect_dispatcher = ConnectDispatcher::new(connector);
        let listen_dispatcher =
            ListenDispatcher::new(listener, settings.listen_dispatcher_settings);

        let fuse = Fuse::new();

        {
            let keychain = keychain.clone();
            let discovery = discovery.clone();
            let view = view.clone();
            let database = database.clone();

            let signup_context = format!("{:?}::processor::signup", view.identifier());
            let signup_listener = listen_dispatcher.register(signup_context);
            let signup_settings = settings.signup;

            fuse.spawn(async move {
                Processor::run_signup(
                    keychain,
                    discovery,
                    view,
                    database,
                    signup_listener,
                    signup_settings,
                )
                .await;
            });
        }

        {
            let keychain = keychain.clone();
            let discovery = discovery.clone();
            let view = view.clone();
            let database = database.clone();

            let prepare_context = format!("{:?}::processor::prepare", view.identifier());
            let prepare_listener = listen_dispatcher.register(prepare_context);

            fuse.spawn(async move {
                Processor::run_prepare(keychain, discovery, view, database, prepare_listener).await;
            });
        }

        {
            let keychain = keychain.clone();
            let discovery = discovery.clone();
            let view = view.clone();
            let database = database.clone();

            let commit_context = format!("{:?}::processor::commit", view.identifier());
            let commit_listener = listen_dispatcher.register(commit_context);

            fuse.spawn(async move {
                Processor::run_commit(keychain, discovery, view, database, commit_listener).await;
            });
        }

        Processor {
            database,
            _fuse: fuse,
        }
    }

    pub fn shutdown(self) -> Database {
        self.database.void()
    }
}

mod commit;
mod prepare;
mod signup;
