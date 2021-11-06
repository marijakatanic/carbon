use crate::lattice::messages::DisclosureSend;

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;
use talk::crypto::Identity;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) enum DisclosureEcho<Instance, Element> {
    Brief {
        origin: Identity,
        disclosure: Hash,
    },
    Expanded {
        origin: Identity,
        disclosure: DisclosureSend<Instance, Element>,
    },
}
