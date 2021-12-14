use buckets::Splittable;

use crate::account::Id;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub id: Id,
    pub height: u64,
}

impl Splittable for Entry {
    type Key = Id;

    fn key(&self) -> Id {
        self.id
    }
}
