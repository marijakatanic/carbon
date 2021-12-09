use crate::{database::Database, discovery::Client, processing::Processor, view::View};

use std::sync::Arc;

use talk::{crypto::KeyChain, net::Listener, sync::voidable::Voidable};

impl Processor {
    pub(in crate::processing) async fn run_commit<L>(
        _keychain: KeyChain,
        _discovery: Arc<Client>,
        _view: View,
        _database: Arc<Voidable<Database>>,
        _listener: L,
    ) where
        L: Listener,
    {
    }
}
