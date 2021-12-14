use buckets::Splittable;

use crate::{
    account::{Entry, Id},
    crypto::Header,
};

use serde::{Deserialize, Serialize};

use talk::crypto::{primitives::hash::Hash, Statement};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Prepare {
    entry: Entry,
    commitment: Hash,
}

impl Prepare {
    pub fn new(entry: Entry, commitment: Hash) -> Self {
        Prepare { entry, commitment }
    }

    pub fn entry(&self) -> Entry {
        self.entry
    }

    pub fn id(&self) -> Id {
        self.entry.id
    }

    pub fn height(&self) -> u64 {
        self.entry.height
    }

    pub fn commitment(&self) -> Hash {
        self.commitment
    }
}

impl Splittable for Prepare {
    type Key = Id;

    fn key(&self) -> Id {
        self.entry.id
    }
}

impl Statement for Prepare {
    type Header = Header;
    const HEADER: Header = Header::Prepare;
}
