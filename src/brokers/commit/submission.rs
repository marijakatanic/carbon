use crate::{
    account::Id,
    commit::{CommitProof, Completion, Payload},
    processing::messages::CommitRequest,
};

use talk::crypto::primitives::hash::Hash;

use zebra::vector::Vector;

pub(in crate::brokers::commit) struct Submission {
    root: Hash,
    commit_proofs: Vec<(Id, CommitProof)>,
    dependencies: Vec<(Id, Completion)>,
    pub requests: Requests,
}

pub(in crate::brokers::commit) struct Requests {
    batch: CommitRequest,
}

impl Submission {
    pub fn new(
        payloads: Vector<Payload>,
        commit_proofs: Vec<(Id, CommitProof)>,
        dependencies: Vec<(Id, Completion)>,
    ) -> Self {
        Submission {
            root: payloads.root(),
            commit_proofs,
            dependencies,
            requests: Requests {
                batch: CommitRequest::Batch(payloads),
            },
        }
    }

    pub fn root(&self) -> Hash {
        self.root
    }
}
