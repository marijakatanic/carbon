use crate::{
    account::Id,
    crypto::{Certificate, Header},
    signup::IdClaim,
};

use serde::{Deserialize, Serialize};

use talk::crypto::{
    primitives::multi::Signature as MultiSignature, KeyCard, KeyChain, Statement as CryptoStatement,
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

impl CryptoStatement for Statement {
    type Header = Header;
    const HEADER: Header = Header::IdAssignment;
}
