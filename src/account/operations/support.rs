use crate::account::Entry;

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Support {
    motion: Hash,
}

impl Support {
    pub fn new(motion: Hash) -> Self {
        Support { motion }
    }

    pub fn motion(&self) -> Hash {
        self.motion
    }

    pub fn dependency(&self) -> Option<Entry> {
        None
    }
}
