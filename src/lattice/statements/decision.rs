use crate::{crypto::Header, lattice::Instance as LatticeInstance};

use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use talk::crypto::primitives::hash;
use talk::crypto::primitives::hash::Hash;
use talk::crypto::Statement;

use zebra::Commitment;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct Decision<Instance> {
    pub view: Commitment,
    pub instance: Instance,
    pub elements: BTreeSet<Hash>,
}

impl<Instance> Decision<Instance>
where
    Instance: LatticeInstance,
{
    pub fn identifier(&self) -> Hash {
        hash::hash(&self).unwrap()
    }
}

impl<Instance> Statement for Decision<Instance>
where
    Instance: LatticeInstance,
{
    type Header = Header;
    const HEADER: Header = Header::LatticeDecision;
}
