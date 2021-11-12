use serde::{Deserialize, Serialize};

use std::hash::Hash;

use talk::crypto::KeyCard;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
