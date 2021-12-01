use crate::prepare::Prepare;

use talk::crypto::primitives::{hash::Hash, multi::Signature as MultiSignature, sign::Signature};

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

    pub fn root(&self) -> Hash {
        self.prepares.root()
    }

    pub fn prepares(&self) -> &[Prepare] {
        self.prepares.items()
    }
}
