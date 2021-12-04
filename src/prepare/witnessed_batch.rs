use crate::{
    crypto::Certificate,
    discovery::Client,
    prepare::{Extract, Prepare, WitnessStatement},
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;

use zebra::vector::Vector;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WitnessedBatch {
    view: Hash,
    prepares: Vector<Prepare>,
    witness: Certificate,
}

#[derive(Doom)]
pub(crate) enum WitnessedBatchError {
    #[doom(description("View unknown"))]
    ViewUnknown,
    #[doom(description("Certificate invalid"))]
    CertificateInvalid,
}

impl WitnessedBatch {
    pub fn new(view: Hash, prepares: Vector<Prepare>, witness: Certificate) -> Self {
        WitnessedBatch {
            view,
            prepares,
            witness,
        }
    }

    pub fn root(&self) -> Hash {
        self.prepares.root()
    }

    pub fn prepares(&self) -> &[Prepare] {
        self.prepares.items()
    }

    pub fn witness(&self) -> &Certificate {
        &self.witness
    }

    pub fn extract(&self, index: usize) -> Extract {
        Extract::new(
            self.view,
            self.prepares.root(),
            self.witness.clone(),
            self.prepares.prove(index),
            self.prepares.items()[index].clone(),
        )
    }

    pub fn validate(&self, discovery: &Client) -> Result<(), Top<WitnessedBatchError>> {
        let view = discovery
            .view(&self.view)
            .ok_or(WitnessedBatchError::ViewUnknown.into_top())
            .spot(here!())?;

        let statement = WitnessStatement::new(self.prepares.root());

        self.witness
            .verify_plurality(&view, &statement)
            .pot(WitnessedBatchError::CertificateInvalid, here!())?;

        Ok(())
    }
}
