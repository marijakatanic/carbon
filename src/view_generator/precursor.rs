use crate::{
    crypto::{Certificate, Identify},
    view_generator::SequenceProposal,
};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::view_generator) struct Precursor {
    pub decisions: Vec<SequenceProposal>,
    pub certificate: Certificate,
}

impl Identify for Precursor {
    fn identifier(&self) -> Hash {
        self.decisions.identifier()
    }
}
