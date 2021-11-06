use crate::lattice::statements::Disclosure;

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::sign::Signature;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct DisclosureSend<Instance, Element> {
    pub disclosure: Disclosure<Instance, Element>,
    pub signature: Signature,
}
