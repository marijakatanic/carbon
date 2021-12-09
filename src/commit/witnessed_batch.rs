use crate::{
    commit::{Extract, Payload, WitnessStatement},
    crypto::Certificate,
    discovery::Client,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;

use zebra::vector::Vector;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WitnessedBatch {
    view: Hash,
    payloads: Vector<Payload>,
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
    pub fn new(view: Hash, payloads: Vector<Payload>, witness: Certificate) -> Self {
        WitnessedBatch {
            view,
            payloads,
            witness,
        }
    }

    pub fn root(&self) -> Hash {
        self.payloads.root()
    }

    pub fn payloads(&self) -> &[Payload] {
        self.payloads.items()
    }

    pub fn witness(&self) -> &Certificate {
        &self.witness
    }

    pub fn extract(&self, index: usize) -> Extract {
        Extract::new(
            self.view,
            self.payloads.root(),
            self.witness.clone(),
            self.payloads.prove(index),
            self.payloads.items()[index].clone(),
        )
    }

    pub fn validate(&self, discovery: &Client) -> Result<(), Top<WitnessedBatchError>> {
        let view = discovery
            .view(&self.view)
            .ok_or(WitnessedBatchError::ViewUnknown.into_top())
            .spot(here!())?;

        let statement = WitnessStatement::new(self.payloads.root());

        self.witness
            .verify_plurality(&view, &statement)
            .pot(WitnessedBatchError::CertificateInvalid, here!())?;

        Ok(())
    }
}
