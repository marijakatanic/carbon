use serde::{Deserialize, Serialize};

use std::hash::Hash;

use talk::crypto::{Identity, KeyCard};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Change {
    Join(KeyCard),
    Leave(KeyCard), // TODO: Refactor to `Leave(Identity)`
}

impl Change {
    pub fn requirement(&self) -> Option<Self> {
        match self {
            Change::Join(_) => None,
            Change::Leave(replica) => Some(Change::Join(replica.clone())),
        }
    }

    pub fn identity(&self) -> Identity {
        match self {
            Change::Join(card) => card.identity(),
            Change::Leave(card) => card.identity(),
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
