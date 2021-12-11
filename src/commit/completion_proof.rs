use crate::{
    commit::{BatchCompletion, Payload},
    discovery::Client,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use zebra::vector::Proof;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CompletionProof {
    batch: BatchCompletion,
    inclusion: Proof,
}

#[derive(Doom)]
pub(crate) enum CompletionProofError {
    #[doom(description("`BatchCompletion` invalid"))]
    BatchCompletionInvalid,
    #[doom(description("Inclusion `Proof` invalid"))]
    InclusionInvalid,
}

impl CompletionProof {
    pub fn new(batch: BatchCompletion, inclusion: Proof) -> Self {
        CompletionProof { batch, inclusion }
    }

    pub fn validate(
        &self,
        discovery: &Client,
        payload: &Payload,
    ) -> Result<(), Top<CompletionProofError>> {
        self.batch
            .validate(discovery)
            .pot(CompletionProofError::BatchCompletionInvalid, here!())?;

        self.inclusion
            .verify(self.batch.root(), payload)
            .pot(CompletionProofError::InclusionInvalid, here!())?;

        Ok(())
    }
}
