use crate::{
    crypto::{Certificate, Identify},
    discovery::Client,
    lattice::{Element as LatticeElement, ElementError as LatticeElementError},
    view::View,
    view_generator::ViewDecision,
};

use doomstack::Top;

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::view_generator) struct SequenceProposal {
    pub proposal: Vec<ViewDecision>, // Ordered by identifier of Thing
    pub proof: Certificate,
}

impl Identify for SequenceProposal {
    fn identifier(&self) -> Hash {
        self.proposal.identifier()
    }
}

impl LatticeElement for SequenceProposal {
    fn validate(&self, client: &Client, view: &View) -> Result<(), Top<LatticeElementError>> {
        todo!()
    }
}
