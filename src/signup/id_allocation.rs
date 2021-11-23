use crate::{
    account::Id,
    crypto::{Header, Identify},
    view::View,
};

use doomstack::{Doom, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;
use talk::crypto::primitives::sign::Signature;
use talk::crypto::{Identity, KeyChain, Statement};

#[derive(Serialize, Deserialize)]
pub(crate) struct IdAllocation {
    assigner: Identity,
    allocation: Allocation,
    signature: Signature,
}

#[derive(Serialize, Deserialize)]
struct Allocation {
    view: Hash,
    id: Id,
    identity: Identity,
}

#[derive(Doom)]
pub(crate) enum IdAllocationError {
    #[doom(description("Foreign view"))]
    ForeignView,
}

impl IdAllocation {
    pub fn new(keychain: &KeyChain, view: &View, identity: Identity, id: Id) -> Self {
        let assigner = keychain.keycard().identity();

        let view = view.identifier();
        let allocation = Allocation { view, id, identity };

        let signature = keychain.sign(&allocation).unwrap();

        IdAllocation {
            assigner,
            allocation,
            signature,
        }
    }

    pub fn validate(&self, view: &View) -> Result<(), Top<IdAllocationError>> {
        todo!()
    }
}

impl Statement for Allocation {
    type Header = Header;
    const HEADER: Header = Header::IdAllocation;
}
