use serde::{Deserialize, Serialize};

use talk::crypto::{primitives::hash::Hash, Identity};

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) enum DisclosureEcho<Element> {
    Brief { origin: Identity, proposal: Hash },
    Expanded { origin: Identity, proposal: Element },
}
