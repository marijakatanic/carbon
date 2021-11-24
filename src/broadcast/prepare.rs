use crate::crypto::Header;

use serde::{Deserialize, Serialize};

use talk::crypto::{primitives::hash::Hash, Statement};

#[derive(Serialize, Deserialize)]
pub(crate) struct Prepare {
    height: u64,
    commitment: Hash,
}

impl Statement for Prepare {
    type Header = Header;
    const HEADER: Header = Header::Prepare;
}
