use crate::{database::Database, processing::Processor, view::View};

use doomstack::{Doom, Top};

use std::sync::Arc;

use talk::net::{Listener, SecureConnection};
use talk::sync::fuse::Fuse;
use talk::sync::voidable::Voidable;

#[derive(Doom)]
enum ServeSignupError {
    #[doom(description("Connection error"))]
    ConnectionError,
}

impl Processor {
    pub(in crate::processing) async fn signup<L>(
        _view: View,
        database: Arc<Voidable<Database>>,
        mut listener: L,
    ) where
        L: Listener,
    {
        let fuse = Fuse::new();

        loop {
            if let Ok((_, connection)) = listener.accept().await {
                let database = database.clone();

                fuse.spawn(async move {
                    let _ = Processor::serve_signup(database, connection).await;
                });
            }
        }
    }

    async fn serve_signup(
        _database: Arc<Voidable<Database>>,
        _connection: SecureConnection,
    ) -> Result<(), Top<ServeSignupError>> {
        loop {}
    }
}
