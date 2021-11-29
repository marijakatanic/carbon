use crate::prepare::Prepare;

use talk::crypto::primitives::{multi::Signature as MultiSignature, sign::Signature};

use zebra::vector::Vector;

pub(crate) struct Batch {
    prepares: Vector<Prepare>,
    batch_signature: MultiSignature,
    individual_signatures: Vec<Option<Signature>>,
}
