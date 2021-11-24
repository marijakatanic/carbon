use crate::{
    account::Id,
    signup::{IdAllocation, IdRequest},
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::{primitives::hash::Hash, Identity};

#[derive(Serialize, Deserialize)]
pub(crate) struct IdClaim {
    request: IdRequest,
    allocation: IdAllocation,
}

#[derive(Doom)]
pub(crate) enum IdClaimError {
    #[doom(description("`IdRequest` invalid"))]
    IdRequestInvalid,
    #[doom(description("`IdAllocation` invalid"))]
    IdAllocationInvalid,
}

impl IdClaim {
    pub fn new(request: IdRequest, allocation: IdAllocation) -> Self {
        IdClaim {
            request,
            allocation,
        }
    }

    pub fn view(&self) -> Hash {
        self.request.view()
    }

    pub fn id(&self) -> Id {
        self.allocation.id()
    }

    pub fn client(&self) -> Identity {
        self.request.client()
    }

    pub fn validate(&self) -> Result<(), Top<IdClaimError>> {
        self.request
            .validate()
            .pot(IdClaimError::IdRequestInvalid, here!())?;

        self.allocation
            .validate(&self.request)
            .pot(IdClaimError::IdAllocationInvalid, here!())?;

        Ok(())
    }
}
