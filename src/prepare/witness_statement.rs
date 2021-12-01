use crate::crypto::Header;

use serde::Serialize;

use talk::crypto::{primitives::hash::Hash, Statement};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WitnessStatement(Hash);

impl WitnessStatement {
    pub fn new(root: Hash) -> Self {
        WitnessStatement(root)
    }
}

impl Statement for WitnessStatement {
    type Header = Header;
    const HEADER: Header = Header::PrepareWitness;
}
