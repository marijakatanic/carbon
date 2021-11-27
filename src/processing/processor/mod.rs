use crate::{crypto::Identify, database::Database, processing::ProcessorSettings, view::View};

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
            let view = view.clone();
            let database = database.clone();

            let signup_context = format!("{:?}::processor::signup", view.identifier());
            let signup_listener = listen_dispatcher.register(signup_context);
            let signup_settings = settings.signup;

            fuse.spawn(async move {
                Processor::run_signup(keychain, view, database, signup_listener, signup_settings)
                    .await;
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

mod signup;
