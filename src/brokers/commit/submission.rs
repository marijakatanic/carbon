use zebra::vector::Vector;

use crate::{
    account::Id,
    commit::{CommitProof, Completion, Payload},
    processing::messages::CommitRequest,
};

pub(in crate::brokers::commit) struct Submission {
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
            commit_proofs,
            dependencies,
            requests: Requests {
                batch: CommitRequest::Batch(payloads),
            },
        }
    }
}
