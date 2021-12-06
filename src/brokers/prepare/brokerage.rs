use crate::{
    brokers::prepare::{BrokerFailure, Reduction, Request},
    prepare::BatchCommit,
};

use tokio::sync::oneshot::Sender;

type ReductionInlet = Sender<Result<Reduction, BrokerFailure>>;
type OutcomeInlet = Sender<Result<BatchCommit, BrokerFailure>>;

pub(in crate::brokers::prepare) struct Brokerage {
    pub request: Request,
    pub reduction_inlet: ReductionInlet,
    pub outcome_inlet: OutcomeInlet,
}
