use serde::{Deserialize, Serialize};

use std::hash::Hash;

use talk::crypto::KeyCard;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Change {
    Join(KeyCard),
    Leave(KeyCard),
}

impl Change {
    pub fn mirror(self) -> Self {
        match self {
            Change::Join(replica) => Change::Leave(replica),
            Change::Leave(replica) => Change::Join(replica),
        }
    }

    pub fn is_join(&self) -> bool {
        match self {
            Change::Join(_) => true,
            Change::Leave(_) => false,
        }
    }

    pub fn is_leave(&self) -> bool {
        match self {
            Change::Join(_) => false,
            Change::Leave(_) => true,
        }
    }
}
