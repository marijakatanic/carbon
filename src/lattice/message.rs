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
    #[doom(description("`Message` contains an `Element` that is not (yet?) safe"))]
    UnsafeElement,
    #[doom(description("`Message` pertains to a foreign `View`"))]
    ForeignView,
    #[doom(description("`Message` pertains to a foreign `Instance`"))]
    ForeignInstance,
    #[doom(description("`Message` cannot be processed during the current `State`"))]
    WrongState,
    #[doom(description("`Message` is a reply to an old or non-existant `Message`"))]
    StaleMessage,
    #[doom(description("`Message` contains an invalid `Signature`"))]
    InvalidSignature,
    #[doom(description("`Message::CertificationRequest` contains no `Element`s"))]
    EmptyCertificationRequest,
    #[doom(description("`Message::CertificationUpdate` contains no `Element`s"))]
    EmptyCertificationUpdate,
    #[doom(description(
        "`Message::CertificationUpdate` contains `Element`s that overlap with `proposed_set`"
    ))]
    OverlappingCertificationUpdate,
}
