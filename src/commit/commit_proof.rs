use crate::{commit::Payload, discovery::Client, prepare::BatchCommit};

use doomstack::{here, Doom, ResultExt, Top};

use zebra::vector::Proof;

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
        payload: &Payload,
    ) -> Result<(), Top<CommitProofError>> {
        self.batch
            .validate(discovery)
            .pot(CommitProofError::BatchCommitInvalid, here!())?;

        let prepare = payload.prepare();

        self.inclusion
            .verify(self.batch.root(), &prepare)
            .pot(CommitProofError::InclusionInvalid, here!())?;

        Ok(())
    }
}
