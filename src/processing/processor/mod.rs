use crate::{crypto::Identify, database::Database, view::View};

use std::sync::Arc;

use talk::crypto::KeyChain;
use talk::link::context::{ConnectDispatcher, ListenDispatcher};
use talk::net::{Connector, Listener};
use talk::sync::fuse::Fuse;
use talk::sync::voidable::Voidable;

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
    ) -> Self
    where
        C: Connector,
        L: Listener,
    {
        let database = Arc::new(Voidable::new(database));

        let _connect_dispatcher = ConnectDispatcher::new(connector);
        let listen_dispatcher = ListenDispatcher::new(listener, Default::default()); // TODO: Forward settings

        let fuse = Fuse::new();

        let signup_context = format!("{:?}::processor::signup", view.identifier(),);
        let signup_listener = listen_dispatcher.register(signup_context);

        {
            let keychain = keychain.clone();
            let view = view.clone();
            let database = database.clone();

            fuse.spawn(async move {
                Processor::signup(keychain, view, database, signup_listener).await;
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
