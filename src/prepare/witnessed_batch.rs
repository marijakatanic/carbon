use crate::{crypto::Certificate, prepare::Prepare};

use zebra::vector::Vector;

pub(crate) struct WitnessedBatch {
    prepares: Vector<Prepare>,
    witness: Certificate,
}

impl WitnessedBatch {
    pub fn new(prepares: Vector<Prepare>, witness: Certificate) -> Self {
        WitnessedBatch { prepares, witness }
    }
}
