use bit_vec::BitVec;

use crate::prepare::{Extract, WitnessedBatch};

use std::iter;

pub(crate) struct BatchHolder {
    batch: WitnessedBatch,
    references: BitVec,
}

impl BatchHolder {
    pub fn new(batch: WitnessedBatch) -> Self {
        let references = iter::repeat(true)
            .take(batch.prepares().len())
            .collect::<BitVec>();

        BatchHolder { batch, references }
    }

    pub fn unref(&mut self, index: usize) {
        self.references.set(index, false);
    }
}
