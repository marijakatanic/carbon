use talk::crypto::primitives::sign::Signature;

pub(crate) enum Attachment {
    Signature(Signature),
    MultiSignature,
}
