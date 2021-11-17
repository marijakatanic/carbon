use crate::{crypto::Identify, view_generator::ViewLatticeBrief};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::view_generator) struct SequenceLatticeBrief {
    pub proposal: Vec<ViewLatticeBrief>, // Sorted by `Identify::identifier()`
}

impl Identify for SequenceLatticeBrief {
    fn identifier(&self) -> Hash {
        self.proposal.identifier()
    }
}
