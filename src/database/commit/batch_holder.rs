use crate::commit::{BatchCompletion, WitnessedBatch};

pub(crate) struct BatchHolder {
    batch: WitnessedBatch,
    applied: bool,
    completion: Option<BatchCompletion>,
}

impl BatchHolder {
    pub fn new(batch: WitnessedBatch) -> Self {
        BatchHolder {
            batch,
            applied: false,
            completion: None,
        }
    }

    pub fn batch(&self) -> &WitnessedBatch {
        &self.batch
    }

    pub fn applied(&self) -> bool {
        self.applied
    }

    pub fn completed(&self) -> bool {
        self.completion.is_some()
    }

    pub fn apply(&mut self) {
        self.applied = true;
    }

    pub fn complete(&mut self, completion: BatchCompletion) {
        self.completion = Some(completion);
    }
}
