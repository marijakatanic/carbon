use crate::prepare::{BatchRoot, Prepare};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::{
    primitives::{hash::Hash, multi::Signature as MultiSignature},
    KeyChain,
};

use zebra::vector::{Proof, Vector};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Reduction {
    root: Hash,
    proof: Proof,
}

#[derive(Doom)]
pub(crate) enum ReductionError {
    #[doom(description("Proof invalid"))]
    ProofInvalid,
}

impl Reduction {
    pub fn batch(prepares: &Vector<Prepare>) -> Vec<Reduction> {
        (0..prepares.len())
            .map(|index| Reduction {
                root: prepares.root(),
                proof: prepares.prove(index),
            })
            .collect()
    }

    pub fn certify(
        &self,
        keychain: &KeyChain,
        prepare: &Prepare,
    ) -> Result<MultiSignature, Top<ReductionError>> {
        self.proof
            .verify(self.root, prepare)
            .pot(ReductionError::ProofInvalid, here!())?;

        Ok(keychain.multisign(&BatchRoot::new(self.root)).unwrap())
    }
}
