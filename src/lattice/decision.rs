use crate::{
    crypto::{Header, Identify},
    lattice::Instance as LatticeInstance,
};

use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use talk::crypto::primitives::hash;
use talk::crypto::primitives::hash::Hash;
use talk::crypto::Statement;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct Decision<Instance> {
    pub view: Hash,
    pub instance: Instance,
    pub elements: BTreeSet<Hash>,
}

impl<Instance> Identify for Decision<Instance>
where
    Instance: LatticeInstance,
{
    fn identifier(&self) -> Hash {
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
