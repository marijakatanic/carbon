use crate::crypto::Header;

use serde::Serialize;

use talk::crypto::{primitives::hash::Hash, Statement};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PrepareBatchRoot(Hash);

impl PrepareBatchRoot {
    pub fn new(root: Hash) -> Self {
        PrepareBatchRoot(root)
    }
}

impl Statement for PrepareBatchRoot {
    type Header = Header;
    const HEADER: Header = Header::PrepareBatchRoot;
}
