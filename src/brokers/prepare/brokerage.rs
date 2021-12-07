use crate::{
    brokers::prepare::{BrokerFailure, Reduction, Request},
    prepare::BatchCommit,
};

use tokio::sync::oneshot::Sender;

type ReductionInlet = Sender<Result<Reduction, BrokerFailure>>;
type CommitInlet = Sender<Result<BatchCommit, BrokerFailure>>;

pub(in crate::brokers::prepare) struct Brokerage {
    pub request: Request,
    pub reduction_inlet: ReductionInlet,
    pub commit_inlet: CommitInlet,
}
