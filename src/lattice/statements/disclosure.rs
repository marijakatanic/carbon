use crate::crypto::Header;

use serde::{Deserialize, Serialize};

use talk::crypto::Statement;

use zebra::Commitment;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct Disclosure<Instance, Element> {
    pub view: Commitment,
    pub instance: Instance,
    pub element: Element,
}

impl<Instance, Element> Statement for Disclosure<Instance, Element>
where
    Instance: Serialize,
    Element: Serialize,
{
    type Header = Header;
    const HEADER: Header = Header::LatticeDisclosureSend;
}
