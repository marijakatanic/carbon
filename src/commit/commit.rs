use crate::{discovery::Client, prepare::BatchCommit};

use doomstack::{here, Doom, ResultExt, Top};

use zebra::vector::Proof;

use super::Payload;

pub(crate) struct Commit {
    batch: BatchCommit,
    inclusion: Proof,
    payload: Payload,
}

#[derive(Doom)]
pub(crate) enum CommitError {
    #[doom(description("Batch commit invalid"))]
    BatchCommitInvalid,
    #[doom(description("Inclusion proof invalid"))]
    InclusionInvalid,
}

impl Commit {
    pub fn new(batch: BatchCommit, inclusion: Proof, payload: Payload) -> Self {
        Commit {
            batch,
            inclusion,
            payload,
        }
    }

    pub fn validate(&self, discovery: &Client) -> Result<(), Top<CommitError>> {
        self.batch
            .validate(discovery)
            .pot(CommitError::BatchCommitInvalid, here!())?;

        let prepare = self.payload.prepare();

        self.inclusion
            .verify(self.batch.root(), &prepare)
            .pot(CommitError::InclusionInvalid, here!())?;

        Ok(())
    }
}
