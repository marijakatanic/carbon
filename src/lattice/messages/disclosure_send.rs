use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) enum DisclosureSend<Element> {
    Brief { proposal: Hash },
    Expanded { proposal: Element },
}
