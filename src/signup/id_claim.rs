use crate::{account::Id, crypto::Rogue, signup::IdAllocation, view::View};

use doomstack::{here, Doom, ResultExt, Top};

use talk::crypto::{Identity, KeyCard};

pub(crate) struct IdClaim {
    keycard: KeyCard,
    allocation: IdAllocation,
    rogue: Rogue,
}

#[derive(Doom)]
pub(crate) enum IdClaimError {
    #[doom(description("`KeyCard` mismatch"))]
    KeyCardMismatch,
    #[doom(description("Invalid `IdAllocation`"))]
    InvalidIdAllocation,
    #[doom(description("Rogue-safety proof invalid"))]
    RogueInvalid,
}

impl IdClaim {
    pub fn new(keycard: KeyCard, allocation: IdAllocation, rogue: Rogue) -> Self {
        IdClaim {
            keycard,
            allocation,
            rogue,
        }
    }

    pub fn id(&self) -> Id {
        self.allocation.id()
    }

    pub fn identity(&self) -> Identity {
        self.allocation.identity()
    }

    pub fn validate(&self, view: &View) -> Result<(), Top<IdClaimError>> {
        if self.keycard.identity() != self.allocation.identity() {
            return IdClaimError::KeyCardMismatch.fail().spot(here!());
        }

        self.allocation
            .validate(view)
            .pot(IdClaimError::InvalidIdAllocation, here!())?;

        self.rogue
            .validate(&self.keycard)
            .pot(IdClaimError::RogueInvalid, here!())?;

        Ok(())
    }
}
