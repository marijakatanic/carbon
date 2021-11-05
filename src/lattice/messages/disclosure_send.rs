use crate::{
    lattice::{statements::Disclosure, LatticeElement},
    view::View,
};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::sign::Signature;
use talk::unicast::Message as UnicastMessage;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct DisclosureSend<Instance, Element> {
    pub disclosure: Disclosure<Instance, Element>,
    pub signature: Signature,
}

impl<Instance, Element> DisclosureSend<Instance, Element>
where
    Instance: UnicastMessage + Clone,
    Element: LatticeElement,
{
    pub fn validate(view: &View, instance: Instance) -> bool {
        true
    }
}
