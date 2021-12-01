use crate::{account::Operation, discovery::Client, prepare::BatchCommit};

use doomstack::{here, Doom, ResultExt, Top};

use zebra::vector::Proof;

pub(crate) struct Commit {
    batch: BatchCommit,
    inclusion: Proof,
}

#[derive(Doom)]
pub(crate) enum CommitError {
    #[doom(description("Batch commit invalid"))]
    BatchCommitInvalid,
    #[doom(description("Inclusion proof invalid"))]
    InclusionInvalid,
}

impl Commit {
    pub fn new(batch: BatchCommit, inclusion: Proof) -> Self {
        Commit { batch, inclusion }
    }

    pub fn validate(
        &self,
        discovery: &Client,
        operation: &Operation,
    ) -> Result<(), Top<CommitError>> {
        self.batch
            .validate(discovery)
            .pot(CommitError::BatchCommitInvalid, here!())?;

        self.inclusion
            .verify(self.batch.root(), operation)
            .pot(CommitError::InclusionInvalid, here!())?;

        Ok(())
    }
}
