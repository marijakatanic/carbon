use buckets::Splittable;

use crate::{account::Id, crypto::Header};

use serde::{Deserialize, Serialize};

use talk::crypto::{primitives::hash::Hash, Statement};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Prepare {
    id: Id,
    height: u64,
    commitment: Hash,
}

impl Prepare {
    pub fn new(id: Id, height: u64, commitment: Hash) -> Self {
        Prepare {
            id,
            height,
            commitment,
        }
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn height(&self) -> u64 {
        self.height
    }

    pub fn commitment(&self) -> Hash {
        self.commitment
    }
}

impl Splittable for Prepare {
    type Key = Id;

    fn key(&self) -> Id {
        self.id
    }
}

impl Statement for Prepare {
    type Header = Header;
    const HEADER: Header = Header::Prepare;
}
