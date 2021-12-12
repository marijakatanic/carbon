use crate::{
    account::Operation,
    commit::{CommitProof, CommitProofError, Payload},
    discovery::Client,
};

use doomstack::Top;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Commit {
    proof: CommitProof,
    payload: Payload,
}

impl Commit {
    pub fn new(proof: CommitProof, payload: Payload) -> Self {
        Commit { proof, payload }
    }

    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    pub fn operation(&self) -> &Operation {
        self.payload.operation()
    }

    pub fn validate(&self, discovery: &Client) -> Result<(), Top<CommitProofError>> {
        let prepare = self.payload.prepare();
        self.proof.validate(&discovery, &prepare)
    }
}
