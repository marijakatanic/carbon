use crate::{
    crypto::{Certificate, Identify},
    discovery::Client,
    lattice::{Decisions, Element as LatticeElement, ElementError as LatticeElementError},
    view::View,
    view_generator::{LatticeInstance, SequenceLatticeBrief, ViewLatticeBrief},
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::view_generator) struct SequenceLatticeElement {
    pub view_lattice_decisions: Vec<ViewLatticeBrief>, // Sorted by `Identify::identifier()`
    pub certificate: Certificate,
}

#[derive(Doom)]
enum SequenceProposalError {
    #[doom(description("Invalid `Certificate`"))]
    InvalidCertificate,
}

impl SequenceLatticeElement {
    pub(in crate::view_generator) fn to_brief(self) -> SequenceLatticeBrief {
        SequenceLatticeBrief {
            view_lattice_decisions: self.view_lattice_decisions,
        }
    }
}

impl LatticeElement for SequenceLatticeElement {
    fn validate(&self, _client: &Client, view: &View) -> Result<(), Top<LatticeElementError>> {
        let decisions = Decisions {
            view: view.identifier(),
            instance: LatticeInstance::ViewLattice,
            elements: self.view_lattice_decisions.iter().map(Identify::identifier).collect(),
        };

        self.certificate
            .verify(view, &decisions)
            .pot(SequenceProposalError::InvalidCertificate, here!())
            .pot(LatticeElementError::ElementInvalid, here!())
    }
}

impl Identify for SequenceLatticeElement {
    fn identifier(&self) -> Hash {
        self.view_lattice_decisions.identifier()
    }
}
