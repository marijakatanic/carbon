use doomstack::{here, Doom, ResultExt, Top};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use talk::crypto::primitives::sign::PublicKey;
use talk::crypto::KeyCard;
use talk::link::rendezvous::Client;

pub(crate) struct Directory {
    client: Arc<Client>,
    database: Arc<Mutex<Database>>,
}

struct Database {
    cache: HashMap<PublicKey, KeyCard>,
}

#[derive(Doom)]
pub(crate) enum DirectoryError {
    #[doom(description("Card unknown"))]
    CardUnknown,
}

impl Directory {
    pub fn new(client: Client) -> Self {
        let client = Arc::new(client);
        let database = Arc::new(Mutex::new(Database {
            cache: HashMap::new(),
        }));

        Directory { client, database }
    }

    pub async fn get_card(&self, root: PublicKey) -> Result<KeyCard, Top<DirectoryError>> {
        match self.search(root) {
            Some(card) => Ok(card),
            None => self
                .client
                .get_card(root)
                .await
                .pot(DirectoryError::CardUnknown, here!())
                .map(|card| self.store(card)),
        }
    }

    fn search(&self, root: PublicKey) -> Option<KeyCard> {
        self.database.lock().unwrap().cache.get(&root).cloned()
    }

    fn store(&self, card: KeyCard) -> KeyCard {
        self.database
            .lock()
            .unwrap()
            .cache
            .insert(card.root(), card.clone());

        card
    }
}
