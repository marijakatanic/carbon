use crate::crypto::Identify;

use talk::crypto::primitives::hash::Hash;

pub(crate) struct SequenceProposal {}

impl Identify for SequenceProposal {
    fn identifier(&self) -> Hash {
        todo!()
    }
}
