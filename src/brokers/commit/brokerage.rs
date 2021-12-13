use crate::{
    account::Id,
    brokers::commit::{BrokerFailure, Request},
    commit::{Commit, CommitProof, Completion, CompletionProof, Payload},
};

use tokio::sync::oneshot::Sender;

type CompletionInlet = Sender<Result<CompletionProof, BrokerFailure>>;

pub(in crate::brokers::commit) struct Brokerage {
    pub request: Request,
    pub completion_inlet: CompletionInlet,
}

pub(in crate::brokers::commit) struct UnzippedBrokerages {
    pub payloads: Vec<Payload>,
    pub commit_proofs: Vec<(Id, CommitProof)>,
    pub dependencies: Vec<(Id, Completion)>,

    pub completion_inlets: Vec<CompletionInlet>,
}

impl Brokerage {
    pub fn unzip(brokerages: Vec<Brokerage>) -> UnzippedBrokerages {
        let mut payloads = Vec::new();
        let mut commit_proofs = Vec::new();
        let mut dependencies = Vec::new();

        let mut completion_inlets = Vec::new();

        for brokerage in brokerages {
            let Request {
                commit: Commit { proof, payload },
                dependency,
            } = brokerage.request;

            let id = payload.id();

            payloads.push(payload);
            commit_proofs.push((id, proof));

            if let Some(dependency) = dependency {
                dependencies.push((id, dependency));
            }

            completion_inlets.push(brokerage.completion_inlet);
        }

        UnzippedBrokerages {
            payloads,
            commit_proofs,
            dependencies,

            completion_inlets,
        }
    }
}
