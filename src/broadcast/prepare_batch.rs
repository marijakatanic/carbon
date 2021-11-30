use crate::broadcast::Prepare;

use talk::crypto::primitives::{multi::Signature as MultiSignature, sign::Signature};

use zebra::vector::Vector;

pub(crate) struct PrepareBatch {
    prepares: Vector<Prepare>,
    root_signature: MultiSignature,
    individual_signatures: Vec<Option<Signature>>,
}

impl PrepareBatch {
    pub fn new(
        prepares: Vector<Prepare>,
        root_signature: MultiSignature,
        individual_signatures: Vec<Option<Signature>>,
    ) -> Self {
        PrepareBatch {
            prepares,
            root_signature,
            individual_signatures,
        }
    }
}
