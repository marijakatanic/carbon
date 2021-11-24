use crate::{
    account::Id,
    crypto::{Header, Identify},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;
use talk::crypto::primitives::sign::Signature;
use talk::crypto::{Identity, KeyChain, Statement};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct IdAllocation {
    assigner: Identity,
    allocation: Allocation,
    signature: Signature,
}

#[derive(Clone, Serialize, Deserialize)]
struct Allocation {
    view: Hash,
    id: Id,
    identity: Identity,
}

#[derive(Doom)]
pub(crate) enum IdAllocationError {
    #[doom(description("Foreign view"))]
    ForeignView,
    #[doom(description("Foreign assigner"))]
    ForeignAssigner,
    #[doom(description("Invalid signature"))]
    InvalidSignature,
    #[doom(description("Assigned `Id` is out of the assigner's allocation range"))]
    IdOutOfRange,
}

impl IdAllocation {
    pub fn new(keychain: &KeyChain, view: &View, id: Id, identity: Identity) -> Self {
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
        if self.allocation.view != view.identifier() {
            return IdAllocationError::ForeignView.fail().spot(here!());
        }

        let keycard = view
            .members()
            .get(&self.assigner)
            .ok_or(IdAllocationError::ForeignAssigner.into_top())
            .spot(here!())?;

        self.signature
            .verify(&keycard, &self.allocation)
            .pot(IdAllocationError::InvalidSignature, here!())?;

        if !view
            .allocation_range(self.assigner)
            .contains(&self.allocation.id)
        {
            return IdAllocationError::IdOutOfRange.fail().spot(here!());
        }

        Ok(())
    }
}

impl Statement for Allocation {
    type Header = Header;
    const HEADER: Header = Header::IdAllocation;
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::view::test::InstallGenerator;

    #[test]
    fn correct() {
        let install_generator = InstallGenerator::new(4);

        let view = install_generator.view(4);

        let assigner = install_generator
            .keychains
            .iter()
            .find(|keychain| {
                keychain.keycard().identity() == *view.members().keys().next().unwrap()
            })
            .cloned()
            .unwrap();

        let user = KeyChain::random().keycard().identity();

        let id_allocation = IdAllocation::new(&assigner, &view, 0, user);
        id_allocation.validate(&view).unwrap();
    }

    #[test]
    fn id_out_of_range() {
        let install_generator = InstallGenerator::new(4);

        let view = install_generator.view(4);

        let assigner = install_generator
            .keychains
            .iter()
            .find(|keychain| {
                keychain.keycard().identity() != *view.members().keys().next().unwrap()
            })
            .cloned()
            .unwrap();

        let user = KeyChain::random().keycard().identity();

        let id_allocation = IdAllocation::new(&assigner, &view, 0, user);
        assert!(id_allocation.validate(&view).is_err());
    }
}
