use crate::{
    crypto::{Header, Identify},
    lattice::{Element as LatticeElement, Instance as LatticeInstance},
};

use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use talk::crypto::primitives::hash::Hash;
use talk::crypto::Statement;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Decisions<Instance> {
    pub(in crate::lattice) view: Hash,
    pub(in crate::lattice) instance: Instance,
    pub(in crate::lattice) elements: BTreeSet<Hash>,
}

impl<Instance> Decisions<Instance> {
    pub fn new<'i, E, I>(view: Hash, instance: Instance, elements: E) -> Self
    where
        E: IntoIterator<Item = &'i I>,
        I: LatticeElement,
    {
        let elements = elements.into_iter().map(Identify::identifier).collect();

        Decisions {
            view,
            instance,
            elements,
        }
    }
}

impl<Instance> Statement for Decisions<Instance>
where
    Instance: LatticeInstance,
{
    type Header = Header;
    const HEADER: Header = Header::LatticeDecisions;
}
