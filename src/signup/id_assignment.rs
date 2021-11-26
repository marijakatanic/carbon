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

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct IdAssignment {
    view: Hash,
    statement: Statement,
    certificate: Certificate,
}

#[derive(Debug, Serialize, Deserialize)]
struct Statement {
    id: Id,
    keycard: KeyCard,
}

pub(crate) struct IdAssignmentAggregator {
    view: Hash,
    aggregator: Aggregator<Statement>,
}

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
            .multisign(&Statement {
                id: claim.id(),
                keycard: claim.client(),
            })
            .unwrap()
    }

    pub fn validate(&self, client: &Client) -> Result<(), Top<IdAssignmentError>> {
        let view = client
            .view(&self.view)
            .ok_or(IdAssignmentError::ViewUnknown.into_top())
            .spot(here!())?;

        self.certificate
            .verify_quorum(&view, &self.statement)
            .pot(IdAssignmentError::CertificateInvalid, here!())?;

        Ok(())
    }
}

impl IdAssignmentAggregator {
    pub fn new(view: View, id: Id, keycard: KeyCard) -> Self {
        let view_identifier = view.identifier();
        let statement = Statement { id, keycard };
        let aggregator = Aggregator::new(view, statement);

        IdAssignmentAggregator {
            view: view_identifier,
            aggregator,
        }
    }

    pub fn add(
        &mut self,
        keycard: &KeyCard,
        signature: MultiSignature,
    ) -> Result<(), Top<MultiError>> {
        self.aggregator.add(keycard, signature)
    }

    pub fn id(&self) -> Id {
        self.aggregator.statement().id
    }

    pub fn keycard(&self) -> KeyCard {
        self.aggregator.statement().keycard.clone()
    }

    pub fn multiplicity(&self) -> usize {
        self.aggregator.multiplicity()
    }

    pub fn finalize(self) -> IdAssignment {
        let view = self.view;
        let (statement, certificate) = self.aggregator.finalize_quorum();

        IdAssignment {
            view,
            statement,
            certificate,
        }
    }
}

impl CryptoStatement for Statement {
    type Header = Header;
    const HEADER: Header = Header::IdAssignment;
}
