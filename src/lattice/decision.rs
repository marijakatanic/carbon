use crate::{crypto::Header, lattice::Instance as LatticeInstance};

use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use talk::crypto::primitives::hash::Hash;
use talk::crypto::Statement;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct Decision<Instance> {
    pub view: Hash,
    pub instance: Instance,
    pub elements: BTreeSet<Hash>,
}

impl<Instance> Statement for Decision<Instance>
where
    Instance: LatticeInstance,
{
    type Header = Header;
    const HEADER: Header = Header::LatticeDecision;
}
