use crate::{
    crypto::Certificate,
    prepare::{Extract, Prepare},
};

use talk::crypto::primitives::hash::Hash;

use zebra::vector::Vector;

pub(crate) struct WitnessedBatch {
    view: Hash,
    prepares: Vector<Prepare>,
    witness: Certificate,
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
}
