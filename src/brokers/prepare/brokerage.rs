use crate::{
    brokers::prepare::{BrokerFailure, Reduction, Request},
    prepare::{BatchCommit, Prepare},
    signup::IdAssignment,
};

use talk::crypto::primitives::sign::Signature;
use tokio::sync::oneshot::Sender;

type ReductionInlet = Sender<Result<Reduction, BrokerFailure>>;
type CommitInlet = Sender<Result<BatchCommit, BrokerFailure>>;

pub(in crate::brokers::prepare) struct Brokerage {
    pub request: Request,
    pub reduction_inlet: ReductionInlet,
    pub commit_inlet: CommitInlet,
}

pub(in crate::brokers::prepare) struct UnzippedBrokerages {
    pub assignments: Vec<IdAssignment>,
    pub prepares: Vec<Prepare>,
    pub individual_signatures: Vec<Option<Signature>>,

    pub reduction_inlets: Vec<ReductionInlet>,
    pub commit_inlets: Vec<CommitInlet>,
}

impl Brokerage {
    pub fn unzip(brokerages: Vec<Brokerage>) -> UnzippedBrokerages {
        let mut assignments = Vec::new();
        let mut prepares = Vec::new();
        let mut individual_signatures = Vec::new();

        let mut reduction_inlets = Vec::new();
        let mut commit_inlets = Vec::new();

        for brokerage in brokerages {
            let Brokerage {
                request:
                    Request {
                        assignment,
                        prepare,
                        signature,
                    },
                reduction_inlet,
                commit_inlet,
            } = brokerage;

            assignments.push(assignment);
            prepares.push(prepare);
            individual_signatures.push(Some(signature));

            reduction_inlets.push(reduction_inlet);
            commit_inlets.push(commit_inlet);
        }

        UnzippedBrokerages {
            assignments,
            prepares,
            individual_signatures,
            reduction_inlets,
            commit_inlets,
        }
    }
}
