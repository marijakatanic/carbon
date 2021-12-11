use buckets::Buckets;

use crate::{
    account::Entry,
    database::commit::{BatchHolder, PayloadHandle},
};

use std::collections::HashMap;

use talk::crypto::primitives::hash::Hash;

pub(crate) struct Commit {
    pub batches: HashMap<Hash, BatchHolder>,
    pub payloads: Buckets<HashMap<Entry, PayloadHandle>>,
}

impl Commit {
    pub fn new() -> Self {
        Commit {
            batches: HashMap::new(),
            payloads: Buckets::new(),
        }
    }
}
