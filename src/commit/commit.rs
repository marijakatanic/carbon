use crate::{
    commit::{CommitProof, CommitProofError, Payload},
    discovery::Client,
};

use doomstack::Top;

pub(crate) struct Commit {
    proof: CommitProof,
    payload: Payload,
}

impl Commit {
    pub fn new(proof: CommitProof, payload: Payload) -> Self {
        Commit { proof, payload }
    }

    pub fn validate(&self, discovery: &Client) -> Result<(), Top<CommitProofError>> {
        self.proof.validate(&discovery, &self.payload)
    }
}
