use crate::crypto::Header;

use serde::Serialize;

use talk::crypto::{primitives::hash::Hash, Statement};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BatchRoot(Hash);

impl BatchRoot {
    pub fn new(root: Hash) -> Self {
        BatchRoot(root)
    }
}

impl Statement for BatchRoot {
    type Header = Header;
    const HEADER: Header = Header::PrepareBatchRoot;
}
