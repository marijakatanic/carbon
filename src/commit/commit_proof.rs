use crate::{
    discovery::Client,
    prepare::{BatchCommit, Prepare},
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use zebra::vector::Proof;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CommitProof {
    batch: BatchCommit,
    inclusion: Proof,
}

#[derive(Doom)]
pub(crate) enum CommitProofError {
    #[doom(description("`BatchCommit` invalid"))]
    BatchCommitInvalid,
    #[doom(description("Inclusion `Proof` invalid"))]
    InclusionInvalid,
}

impl CommitProof {
    pub fn new(batch: BatchCommit, inclusion: Proof) -> Self {
        CommitProof { batch, inclusion }
    }

    pub fn validate(
        &self,
        discovery: &Client,
        prepare: &Prepare,
    ) -> Result<(), Top<CommitProofError>> {
        self.batch
            .validate(discovery)
            .pot(CommitProofError::BatchCommitInvalid, here!())?;

        self.inclusion
            .verify(self.batch.root(), prepare)
            .pot(CommitProofError::InclusionInvalid, here!())?;

        Ok(())
    }
}
