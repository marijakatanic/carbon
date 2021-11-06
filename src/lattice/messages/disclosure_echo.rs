use crate::lattice::messages::DisclosureSend;

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) enum DisclosureEcho<Instance, Element> {
    Brief(Hash),
    Expanded(DisclosureSend<Instance, Element>),
}
