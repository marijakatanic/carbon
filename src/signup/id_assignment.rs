use crate::{
    account::Id,
    crypto::{Aggregator, Certificate, Header},
    signup::IdClaim,
    view::View,
};

use doomstack::Top;

use serde::{Deserialize, Serialize};

use talk::crypto::{
    primitives::multi::{MultiError, Signature as MultiSignature},
    KeyCard, KeyChain, Statement as CryptoStatement,
};

#[derive(Serialize, Deserialize)]
pub(crate) struct IdAssignment {
    statement: Statement,
    certificate: Certificate,
}

#[derive(Serialize, Deserialize)]
struct Statement {
    id: Id,
    keycard: KeyCard,
}

pub(crate) struct IdAssignmentAggregator(Aggregator<Statement>);

impl IdAssignment {
    pub fn certify(keychain: &KeyChain, claim: &IdClaim) -> MultiSignature {
        keychain
            .multisign(&Statement {
                id: claim.id(),
                keycard: claim.client(),
            })
            .unwrap()
    }
}

impl IdAssignmentAggregator {
    pub fn new(view: View, id: Id, keycard: KeyCard) -> Self {
        let statement = Statement { id, keycard };

        IdAssignmentAggregator(Aggregator::new(view, statement))
    }

    pub fn add(
        &mut self,
        keycard: &KeyCard,
        signature: MultiSignature,
    ) -> Result<(), Top<MultiError>> {
        self.0.add(keycard, signature)
    }

    pub fn multiplicity(&self) -> usize {
        self.0.multiplicity()
    }

    pub fn finalize(self) -> IdAssignment {
        let (statement, certificate) = self.0.finalize_plurality();

        IdAssignment {
            statement,
            certificate,
        }
    }
}

impl CryptoStatement for Statement {
    type Header = Header;
    const HEADER: Header = Header::IdAssignment;
}
