use crate::{database::Database, view::View};

use std::sync::Arc;

use talk::net::{Connector, Listener};
use talk::sync::fuse::Fuse;
use talk::sync::lenders::AtomicLender;

pub(crate) struct Processor {
    database: Arc<AtomicLender<Database>>,
    _fuse: Fuse,
}

impl Processor {
    pub fn new<C, L>(_view: View, _database: Database, _connector: C, _listener: L) -> Self
    where
        C: Connector,
        L: Listener,
    {
        todo!()
    }

    pub fn shutdown(self) -> Database {
        self.database.take()
    }
}
