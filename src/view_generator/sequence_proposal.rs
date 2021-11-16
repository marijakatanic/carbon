use crate::{
    crypto::{Certificate, Identify},
    discovery::Client,
    lattice::{Decisions, Element as LatticeElement, ElementError as LatticeElementError},
    view::View,
    view_generator::{LatticeInstance, ViewDecision},
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::view_generator) struct SequenceProposal {
    pub proposal: Vec<ViewDecision>, // Sorted by `Identify::identifier()`
    pub certificate: Certificate,
}

#[derive(Doom)]
enum SequenceProposalError {
    #[doom(description("Invalid `Certificate`"))]
    InvalidCertificate,
}

impl LatticeElement for SequenceProposal {
    fn validate(&self, _client: &Client, view: &View) -> Result<(), Top<LatticeElementError>> {
        let decisions = Decisions {
            view: view.identifier(),
            instance: LatticeInstance::ViewLattice,
            elements: self.proposal.iter().map(Identify::identifier).collect(),
        };

        self.certificate
            .verify(view, &decisions)
            .pot(SequenceProposalError::InvalidCertificate, here!())
            .pot(LatticeElementError::ElementInvalid, here!())
    }
}

impl Identify for SequenceProposal {
    fn identifier(&self) -> Hash {
        self.proposal.identifier()
    }
}
