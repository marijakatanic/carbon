use crate::commit::{BatchCompletion, WitnessedBatch};

pub(crate) struct BatchHolder {
    batch: WitnessedBatch,
    completion: Option<BatchCompletion>,
}

impl BatchHolder {
    pub fn new(batch: WitnessedBatch) -> Self {
        BatchHolder {
            batch,
            completion: None,
        }
    }

    pub fn batch(&self) -> &WitnessedBatch {
        &self.batch
    }

    pub fn completion(&self) -> Option<&BatchCompletion> {
        self.completion.as_ref()
    }

    pub fn attach(&mut self, completion: BatchCompletion) {
        self.completion = Some(completion);
    }
}
