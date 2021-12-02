use crate::{
    account::Id,
    crypto::{Aggregator, Certificate, Header, Identify},
    discovery::Client,
    signup::IdClaim,
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::{
    primitives::{
        hash::Hash,
        multi::{MultiError, Signature as MultiSignature},
    },
    KeyCard, KeyChain, Statement as CryptoStatement,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IdAssignment {
    view: Hash,
    assignment: Assignment,
    certificate: Certificate,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Assignment {
    id: Id,
    keycard: KeyCard,
}

pub(crate) struct IdAssignmentAggregator(Aggregator<Assignment>);

#[derive(Doom)]
pub(crate) enum IdAssignmentError {
    #[doom(description("Assignment signed in an unknown `View`"))]
    ViewUnknown,
    #[doom(description("Certificate invalid"))]
    CertificateInvalid,
}

impl IdAssignment {
    pub fn certify(keychain: &KeyChain, claim: &IdClaim) -> MultiSignature {
        keychain
            .multisign(&Assignment {
                id: claim.id(),
                keycard: claim.client(),
            })
            .unwrap()
    }

    pub fn id(&self) -> Id {
        self.assignment.id
    }

    pub fn keycard(&self) -> &KeyCard {
        &self.assignment.keycard
    }

    pub fn validate(&self, discovery: &Client) -> Result<(), Top<IdAssignmentError>> {
        let view = discovery
            .view(&self.view)
            .ok_or(IdAssignmentError::ViewUnknown.into_top())
            .spot(here!())?;

        self.certificate
            .verify_quorum(&view, &self.assignment)
            .pot(IdAssignmentError::CertificateInvalid, here!())?;

        Ok(())
    }
}

impl IdAssignmentAggregator {
    pub fn new(view: View, id: Id, keycard: KeyCard) -> Self {
        let statement = Assignment { id, keycard };
        let aggregator = Aggregator::new(view, statement);

        IdAssignmentAggregator(aggregator)
    }

    pub fn add(
        &mut self,
        keycard: &KeyCard,
        signature: MultiSignature,
    ) -> Result<(), Top<MultiError>> {
        self.0.add(keycard, signature)
    }

    pub fn id(&self) -> Id {
        self.0.statement().id
    }

    pub fn keycard(&self) -> KeyCard {
        self.0.statement().keycard.clone()
    }

    pub fn multiplicity(&self) -> usize {
        self.0.multiplicity()
    }

    pub fn finalize(self) -> IdAssignment {
        let view = self.0.view().identifier();
        let (assignment, certificate) = self.0.finalize_quorum();

        IdAssignment {
            view,
            assignment,
            certificate,
        }
    }
}

impl CryptoStatement for Assignment {
    type Header = Header;
    const HEADER: Header = Header::IdAssignment;
}
