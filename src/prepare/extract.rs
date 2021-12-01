use crate::{
    account::Id,
    crypto::Certificate,
    discovery::Client,
    prepare::{Prepare, WitnessStatement},
};

use doomstack::{here, Doom, ResultExt, Top};

use talk::crypto::primitives::hash::Hash;

use zebra::vector::Proof;

pub(crate) struct Extract {
    view: Hash,
    root: Hash,
    witness: Certificate,
    inclusion: Proof,
    prepare: Prepare,
}

#[derive(Doom)]
pub(crate) enum ExtractError {
    #[doom(description("View unknown"))]
    ViewUnknown,
    #[doom(description("Witness invalid"))]
    WitnessInvalid,
    #[doom(description("Proof invalid"))]
    ProofInvalid,
}

impl Extract {
    pub fn new(
        view: Hash,
        root: Hash,
        witness: Certificate,
        inclusion: Proof,
        prepare: Prepare,
    ) -> Self {
        Extract {
            view,
            root,
            witness,
            inclusion,
            prepare,
        }
    }

    pub fn id(&self) -> Id {
        self.prepare.id()
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
            .verify(self.root, &self.prepare)
            .pot(ExtractError::ProofInvalid, here!())?;

        Ok(())
    }
}
