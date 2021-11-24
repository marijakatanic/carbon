use crate::{
    crypto::{Header, Identify},
    lattice::Instance as LatticeInstance,
};

use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use talk::crypto::{primitives::hash::Hash, Statement};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Decision<Instance> {
    pub view: Hash,
    pub instance: Instance,
    pub elements: BTreeSet<Hash>,
}

impl<Instance> Decision<Instance> {
    pub fn new<'f, E, F>(view: Hash, instance: Instance, elements: E) -> Self
    where
        E: IntoIterator<Item = &'f F>,
        F: 'f + Identify,
    {
        let elements = elements.into_iter().map(Identify::identifier).collect();

        Decision {
            view,
            instance,
            elements,
        }
    }
}

impl<Instance> Statement for Decision<Instance>
where
    Instance: LatticeInstance,
{
    type Header = Header;
    const HEADER: Header = Header::LatticeDecisions;
}
