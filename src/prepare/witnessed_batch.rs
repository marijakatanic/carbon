use crate::{crypto::Certificate, prepare::Prepare};

use talk::crypto::primitives::hash::Hash;

use zebra::vector::Vector;

pub(crate) struct WitnessedBatch {
    prepares: Vector<Prepare>,
    witness: Certificate,
}

impl WitnessedBatch {
    pub fn new(prepares: Vector<Prepare>, witness: Certificate) -> Self {
        WitnessedBatch { prepares, witness }
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
}
