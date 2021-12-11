use crate::{
    commit::{CompletionProof, CompletionProofError, Payload},
    discovery::Client,
};

use doomstack::Top;

pub(crate) struct Completion {
    proof: CompletionProof,
    payload: Payload,
}

impl Completion {
    pub fn new(proof: CompletionProof, payload: Payload) -> Self {
        Completion { proof, payload }
    }

    pub fn validate(&self, discovery: &Client) -> Result<(), Top<CompletionProofError>> {
        self.proof.validate(discovery, &self.payload)
    }
}
