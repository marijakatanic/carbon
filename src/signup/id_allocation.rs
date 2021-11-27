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
    client: Identity,
}

#[derive(Doom)]
pub(crate) enum IdAllocationError {
    #[doom(description("Invalid signature"))]
    InvalidSignature,
    #[doom(description("Assigned `Id` is out of the allocator's allocation range"))]
    IdOutOfRange,
}

impl IdAllocation {
    pub fn new(keychain: &KeyChain, request: &IdRequest, id: Id) -> Self {
        let view = request.view();
        let client = request.client().identity();

        let allocation = Allocation { view, id, client };
        let signature = keychain.sign(&allocation).unwrap();

        IdAllocation { id, signature }
    }

    pub fn id(&self) -> Id {
        self.id
    }

    // In order to avoid panics, `request` must have been validated beforehand
    pub fn validate(&self, request: &IdRequest) -> Result<(), Top<IdAllocationError>> {
        let view = View::get(request.view()).unwrap();
        let keycard = view.members().get(&request.allocator()).unwrap();

        let allocation = Allocation {
            view: request.view(),
            id: self.id,
            client: request.client().identity(),
        };

        self.signature
            .verify(&keycard, &allocation)
            .pot(IdAllocationError::InvalidSignature, here!())?;

        if !view
            .allocation_range(request.allocator())
            .contains(&self.id)
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

    use crate::{signup::SignupSettings, view::test::InstallGenerator};

    #[test]
    fn correct() {
        let install_generator = InstallGenerator::new(4);

        let view = install_generator.view(4);

        let allocator = install_generator
            .keychains
            .iter()
            .find(|keychain| {
                keychain.keycard().identity() == *view.members().keys().next().unwrap()
            })
            .cloned()
            .unwrap();

        let client = KeyChain::random();
        let request = IdRequest::new(
            &client,
            &view,
            allocator.keycard().identity(),
            SignupSettings::default().work_difficulty,
        );

        let allocation = IdAllocation::new(&allocator, &request, 0);
        allocation.validate(&request).unwrap();
    }

    #[test]
    fn id_out_of_range() {
        let install_generator = InstallGenerator::new(4);

        let view = install_generator.view(4);

        let allocator = install_generator
            .keychains
            .iter()
            .find(|keychain| {
                keychain.keycard().identity() != *view.members().keys().next().unwrap()
            })
            .cloned()
            .unwrap();

        let client = KeyChain::random();
        let request = IdRequest::new(
            &client,
            &view,
            allocator.keycard().identity(),
            SignupSettings::default().work_difficulty,
        );

        let allocation = IdAllocation::new(&allocator, &request, 0);
        assert!(allocation.validate(&request).is_err());
    }
}
