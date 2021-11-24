use crate::crypto::Identify;

use serde::{Deserialize, Serialize};

use std::hash::Hash as StdHash;

use talk::crypto::{
    primitives::{hash, hash::Hash},
    KeyCard,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, StdHash, Serialize, Deserialize)]
pub(crate) enum Change {
    Join(KeyCard),
    Leave(KeyCard), // TODO: Refactor to `Leave(Identity)`
}

impl Change {
    pub fn keycard(&self) -> KeyCard {
        match self {
            Change::Join(keycard) => keycard.clone(),
            Change::Leave(keycard) => keycard.clone(),
        }
    }
}

impl Identify for Change {
    fn identifier(&self) -> Hash {
        hash::hash(&self).unwrap()
    }
}
