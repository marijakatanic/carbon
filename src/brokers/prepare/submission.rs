use crate::{prepare::Prepare, signup::IdAssignment};
use talk::crypto::primitives::{multi::Signature as MultiSignature, sign::Signature};

use zebra::vector::Vector;

pub(in crate::brokers::prepare) struct Submission {
    pub assignments: Vec<IdAssignment>,
    pub prepares: Vector<Prepare>,
    pub reduction_signature: MultiSignature,
    pub individual_signatures: Vec<Option<Signature>>,
}
