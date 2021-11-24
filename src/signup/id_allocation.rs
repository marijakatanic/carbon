use crate::{account::Id, crypto::Header, signup::IdRequest, view::View};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::{
    primitives::{hash::Hash, sign::Signature},
    Identity, KeyChain, Statement,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IdAllocation {
    id: Id,
    signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Allocation {
    view: Hash,
    id: Id,
    identity: Identity,
}

#[derive(Doom)]
pub(crate) enum IdAllocationError {
    #[doom(description("Invalid signature"))]
    InvalidSignature,
    #[doom(description("Assigned `Id` is out of the assigner's allocation range"))]
    IdOutOfRange,
}

impl IdAllocation {
    pub fn new(keychain: &KeyChain, request: &IdRequest, id: Id) -> Self {
        let view = request.view();
        let identity = request.identity();

        let allocation = Allocation { view, id, identity };
        let signature = keychain.sign(&allocation).unwrap();

        IdAllocation { id, signature }
    }

    pub fn id(&self) -> Id {
        self.id
    }

    // In order to avoid panics, `request` must have been validated beforehand
    pub fn validate(&self, request: &IdRequest) -> Result<(), Top<IdAllocationError>> {
        let view = View::get(request.view()).unwrap();
        let keycard = view.members().get(&request.assigner()).unwrap();

        let allocation = Allocation {
            view: request.view(),
            id: self.id,
            identity: request.identity(),
        };

        self.signature
            .verify(&keycard, &allocation)
            .pot(IdAllocationError::InvalidSignature, here!())?;

        if !view.allocation_range(request.assigner()).contains(&self.id) {
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

        let user = KeyChain::random();
        let id_request = IdRequest::new(&user, &view, assigner.keycard().identity());

        let id_allocation = IdAllocation::new(&assigner, &id_request, 0);
        id_allocation.validate(&id_request).unwrap();
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

        let user = KeyChain::random();
        let id_request = IdRequest::new(&user, &view, assigner.keycard().identity());

        let id_allocation = IdAllocation::new(&assigner, &id_request, 0);
        assert!(id_allocation.validate(&id_request).is_err());
    }
}
