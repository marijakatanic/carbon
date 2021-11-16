use crate::{
    crypto::{Certificate, Identify},
    view_generator::ViewDecision,
};

use talk::crypto::primitives::hash::Hash;

pub(crate) struct SequenceProposal {
    proposal: Vec<ViewDecision>, // Ordered by identifier of Thing
    proof: Certificate,
}

impl Identify for SequenceProposal {
    fn identifier(&self) -> Hash {
        self.proposal.identifier()
    }
}
