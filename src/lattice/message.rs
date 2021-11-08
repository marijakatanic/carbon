use crate::lattice::messages::{DisclosureEcho, DisclosureReady, DisclosureSend};

use doomstack::Doom;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) enum Message<Element> {
    DisclosureSend(DisclosureSend<Element>),
    DisclosureEcho(DisclosureEcho<Element>),
    DisclosureReady(DisclosureReady<Element>),
}

#[derive(Doom)]
pub(in crate::lattice) enum MessageError {
    #[doom(description("`Message` contains an invalid `Element`"))]
    InvalidElement,
}
