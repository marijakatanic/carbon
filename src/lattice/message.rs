use crate::lattice::messages::DisclosureSend;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) enum Message<Instance, Element> {
    DisclosureSend(DisclosureSend<Instance, Element>),
}
