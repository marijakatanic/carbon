use crate::{account::Id, crypto::Certificate};

use std::collections::BTreeSet;

use talk::crypto::primitives::hash::Hash;

pub(crate) struct BatchCompletion {
    view: Hash,
    root: Hash,
    exceptions: BTreeSet<Id>,
    certificate: Certificate,
}
