use crate::{
    account::{Entry, Id, Operation},
    commit::{Payload, WitnessStatement},
    crypto::Certificate,
    discovery::Client,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;

use zebra::vector::Proof;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Extract {
    view: Hash,
    root: Hash,
    witness: Certificate,
    inclusion: Proof,
    payload: Payload,
}

#[derive(Doom)]
pub(crate) enum ExtractError {
    #[doom(description("`View` unknown"))]
    ViewUnknown,
    #[doom(description("Witness invalid"))]
    WitnessInvalid,
    #[doom(description("Inclusion `Proof` invalid"))]
    InclusionProofInvalid,
}

impl Extract {
    pub fn new(
        view: Hash,
        root: Hash,
        witness: Certificate,
        inclusion: Proof,
        payload: Payload,
    ) -> Self {
        Extract {
            view,
            root,
            witness,
            inclusion,
            payload,
        }
    }

    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    pub fn id(&self) -> Id {
        self.payload.id()
    }

    pub fn height(&self) -> u64 {
        self.payload.height()
    }

    pub fn entry(&self) -> Entry {
        self.payload.entry()
    }

    pub fn operation(&self) -> &Operation {
        self.payload.operation()
    }

    pub fn validate(&self, discovery: &Client) -> Result<(), Top<ExtractError>> {
        let view = discovery
            .view(&self.view)
            .ok_or(ExtractError::ViewUnknown.into_top())
            .spot(here!())?;

        let statement = WitnessStatement::new(self.root);

        self.witness
            .verify_plurality(&view, &statement)
            .pot(ExtractError::WitnessInvalid, here!())?;

        self.inclusion
            .verify(self.root, &self.payload)
            .pot(ExtractError::InclusionProofInvalid, here!())?;

        Ok(())
    }
}
