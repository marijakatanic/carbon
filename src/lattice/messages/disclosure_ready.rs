use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;
use talk::crypto::Identity;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) enum DisclosureReady<Element> {
    Brief { origin: Identity, proposal: Hash },
    Expanded { origin: Identity, proposal: Element },
}
