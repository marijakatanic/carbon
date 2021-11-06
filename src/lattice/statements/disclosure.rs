use crate::{
    crypto::Header,
    lattice::{Element as LatticeElement, Instance as LatticeInstance},
};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash;
use talk::crypto::primitives::hash::Hash;
use talk::crypto::Statement;

use zebra::Commitment;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct Disclosure<Instance, Element> {
    pub view: Commitment,
    pub instance: Instance,
    pub element: Element,
}

impl<Instance, Element> Disclosure<Instance, Element>
where
    Instance: LatticeInstance,
    Element: LatticeElement,
{
    pub fn identifier(&self) -> Hash {
        hash::hash(self).unwrap()
    }
}

impl<Instance, Element> Statement for Disclosure<Instance, Element>
where
    Instance: Serialize,
    Element: Serialize,
{
    type Header = Header;
    const HEADER: Header = Header::LatticeDisclosureSend;
}
