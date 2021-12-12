use bit_vec::BitVec;

use crate::prepare::{BatchCommit, Extract, WitnessedBatch};

use std::iter;

pub(crate) struct BatchHolder {
    batch: WitnessedBatch,
    references: BitVec,
    commit: Option<BatchCommit>,
}

impl BatchHolder {
    pub fn new(batch: WitnessedBatch) -> Self {
        let references = iter::repeat(true)
            .take(batch.prepares().len())
            .collect::<BitVec>();

        BatchHolder {
            batch,
            references,
            commit: None,
        }
    }

    pub fn extract(&self, index: usize) -> Extract {
        self.batch.extract(index)
    }

    pub fn commit(&self) -> Option<&BatchCommit> {
        self.commit.as_ref()
    }

    pub fn attach(&mut self, commit: BatchCommit) {
        self.commit = Some(commit);
    }

    pub fn unref(&mut self, index: usize) {
        self.references.set(index, false);
    }
}
