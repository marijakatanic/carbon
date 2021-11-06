use crate::lattice::messages::{DisclosureEcho, DisclosureSend};

use doomstack::Doom;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) enum Message<Instance, Element> {
    DisclosureSend(DisclosureSend<Instance, Element>),
    DisclosureEcho(DisclosureEcho<Instance, Element>),
}

#[derive(Doom)]
pub(in crate::lattice) enum MessageError {
    #[doom(description("`Message` pertains to a foreign `View`"))]
    ForeignView,
    #[doom(description("`Message` pertains to a foreign lattice instance"))]
    ForeignInstance,
    #[doom(description("`Message` incorrectly signed"))]
    IncorrectSignature,
    #[doom(description("`Message` contains an invalid `Element`"))]
    InvalidElement,
}
