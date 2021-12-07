use crate::prepare::{Prepare, ReductionStatement};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::{
    primitives::{hash::Hash, multi::Signature as MultiSignature},
    KeyChain,
};

use zebra::vector::{Proof, Vector};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Inclusion {
    root: Hash,
    proof: Proof,
}

#[derive(Doom)]
pub(crate) enum InclusionError {
    #[doom(description("`Proof` invalid"))]
    ProofInvalid,
}

impl Inclusion {
    pub fn batch(prepares: &Vector<Prepare>) -> Vec<Inclusion> {
        (0..prepares.len())
            .map(|index| Inclusion {
                root: prepares.root(),
                proof: prepares.prove(index),
            })
            .collect()
    }

    pub fn root(&self) -> Hash {
        self.root
    }

    pub fn certify_reduction(
        &self,
        keychain: &KeyChain,
        prepare: &Prepare,
    ) -> Result<MultiSignature, Top<InclusionError>> {
        self.proof
            .verify(self.root, prepare)
            .pot(InclusionError::ProofInvalid, here!())?;

        Ok(keychain
            .multisign(&ReductionStatement::new(self.root))
            .unwrap())
    }
}
