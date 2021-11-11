use crate::lattice::messages::{
    CertificationConfirmation, CertificationRequest, CertificationUpdate, DisclosureEcho,
    DisclosureReady, DisclosureSend,
};

use doomstack::Doom;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) enum Message<Element> {
    DisclosureSend(DisclosureSend<Element>),
    DisclosureEcho(DisclosureEcho<Element>),
    DisclosureReady(DisclosureReady<Element>),
    CertificationRequest(CertificationRequest),
    CertificationConfirmation(CertificationConfirmation),
    CertificationUpdate(CertificationUpdate),
}

#[derive(Doom)]
pub(in crate::lattice) enum MessageError {
    #[doom(description("`Message` contains an invalid `Element`"))]
    InvalidElement,
    #[doom(description("`Message` pertains to a different `Instance`"))]
    WrongInstance,
    #[doom(description("`Message` pertains to a different `View`"))]
    WrongView,
    #[doom(description("`Message` cannot be processed during the current `State`"))]
    WrongState,
    #[doom(description("`Message` is a reply to an old of non-existant `Message`"))]
    StaleMessage,
    #[doom(description("`Message` contains an invalid `Signature`"))]
    InvalidSignature,
    #[doom(description("`Message` contains no new `Element`"))]
    NoNewElements,
    #[doom(description("`Message`'s `Decision` must contain at least one `Element`"))]
    EmptyDecision,
}
