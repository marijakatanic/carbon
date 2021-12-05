use crate::{prepare::Prepare, processing::messages::PrepareRequest, signup::IdAssignment};
use talk::crypto::primitives::{multi::Signature as MultiSignature, sign::Signature};

use zebra::vector::Vector;

pub(in crate::brokers::prepare) struct Submission {
    pub assignments: Vec<IdAssignment>,
    pub requests: Requests,
}

pub(in crate::brokers::prepare) struct Requests {
    pub batch: PrepareRequest,
    pub signatures: PrepareRequest,
}

impl Submission {
    pub fn new(
        assignments: Vec<IdAssignment>,
        prepares: Vector<Prepare>,
        reduction_signature: MultiSignature,
        individual_signatures: Vec<Option<Signature>>,
    ) -> Self {
        Submission {
            assignments,
            requests: Requests {
                batch: PrepareRequest::Batch(prepares),
                signatures: PrepareRequest::Signatures(reduction_signature, individual_signatures),
            },
        }
    }
}
