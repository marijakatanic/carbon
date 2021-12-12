use crate::{
    brokers::commit::{BrokerFailure, Request},
    commit::Completion,
};

use tokio::sync::oneshot::Sender;

type CompletionInlet = Sender<Result<Completion, BrokerFailure>>;

pub(in crate::brokers::commit) struct Brokerage {
    pub request: Request,
    pub completion_inlet: CompletionInlet,
}
