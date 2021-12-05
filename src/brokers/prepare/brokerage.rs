use crate::brokers::prepare::{Failure, Reduction, Request};

use tokio::sync::oneshot::Sender;

type ReductionInlet = Sender<Result<Reduction, Failure>>;

pub(in crate::brokers::prepare) struct Brokerage {
    pub request: Request,
    pub reduction_inlet: ReductionInlet,
}
