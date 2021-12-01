use crate::prepare::Prepare;

use talk::crypto::primitives::{multi::Signature as MultiSignature, sign::Signature};

use zebra::vector::Vector;

pub(crate) struct Batch {
    prepares: Vector<Prepare>,
    root_signature: MultiSignature,
    individual_signatures: Vec<Option<Signature>>,
}

impl Batch {
    pub fn new(
        prepares: Vector<Prepare>,
        root_signature: MultiSignature,
        individual_signatures: Vec<Option<Signature>>,
    ) -> Self {
        Batch {
            prepares,
            root_signature,
            individual_signatures,
        }
    }
}
