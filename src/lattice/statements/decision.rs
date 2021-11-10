use crate::lattice::Instance as LatticeInstance;

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash;
use talk::crypto::primitives::hash::Hash;

use zebra::Commitment;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct Decision<Instance> {
    pub view: Commitment,
    pub instance: Instance,
    pub elements: Vec<Hash>,
}

impl<Instance> Decision<Instance>
where
    Instance: LatticeInstance,
{
    pub fn identifier(&self) -> Hash {
        hash::hash(&self).unwrap()
    }
}
