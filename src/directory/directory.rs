use doomstack::{here, Doom, ResultExt, Top};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use talk::crypto::{Identity, KeyCard};
use talk::link::rendezvous::Client;

pub(crate) struct Directory {
    client: Arc<Client>,
    database: Arc<Mutex<Database>>,
}

struct Database {
    cache: HashMap<Identity, KeyCard>,
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

    pub async fn get_card(&self, identity: Identity) -> Result<KeyCard, Top<DirectoryError>> {
        match self.search(identity) {
            Some(card) => Ok(card),
            None => self
                .client
                .get_card(identity)
                .await
                .pot(DirectoryError::CardUnknown, here!())
                .map(|card| self.store(card)),
        }
    }

    fn search(&self, identity: Identity) -> Option<KeyCard> {
        self.database.lock().unwrap().cache.get(&identity).cloned()
    }

    fn store(&self, card: KeyCard) -> KeyCard {
        self.database
            .lock()
            .unwrap()
            .cache
            .insert(card.identity(), card.clone());

        card
    }
}
