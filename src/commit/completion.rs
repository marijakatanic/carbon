use crate::{
    account::{Entry, Id, Operation},
    commit::{CompletionProof, CompletionProofError, Payload},
    discovery::Client,
};

use doomstack::Top;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Completion {
    proof: CompletionProof,
    payload: Payload,
}

impl Completion {
    pub fn new(proof: CompletionProof, payload: Payload) -> Self {
        Completion { proof, payload }
    }

    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    pub fn entry(&self) -> Entry {
        self.payload.entry()
    }

    pub fn id(&self) -> Id {
        self.payload.id()
    }

    pub fn height(&self) -> u64 {
        self.payload.height()
    }

    pub fn operation(&self) -> &Operation {
        self.payload.operation()
    }

    pub fn validate(&self, discovery: &Client) -> Result<(), Top<CompletionProofError>> {
        self.proof.validate(discovery, &self.payload)
    }
}
