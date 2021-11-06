use crate::lattice::{
    statements::Disclosure, Element as LatticeElement, Instance as LatticeInstance,
};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;
use talk::crypto::primitives::sign::Signature;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct DisclosureSend<Instance, Element> {
    pub disclosure: Disclosure<Instance, Element>,
    pub signature: Signature,
}

impl<Instance, Element> DisclosureSend<Instance, Element>
where
    Instance: LatticeInstance,
    Element: LatticeElement,
{
    pub fn identifier(&self) -> Hash {
        self.disclosure.identifier()
    }
}
