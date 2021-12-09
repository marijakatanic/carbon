use crate::{account::Id, crypto::Header};

use serde::Serialize;

use std::collections::BTreeSet;

use talk::crypto::{primitives::hash::Hash, Statement};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BatchCompletionStatement {
    view: Hash,
    root: Hash,
    exceptions: BTreeSet<Id>,
}

impl BatchCompletionStatement {
    pub fn new(view: Hash, root: Hash, exceptions: BTreeSet<Id>) -> Self {
        BatchCompletionStatement {
            view,
            root,
            exceptions,
        }
    }
}

impl Statement for BatchCompletionStatement {
    type Header = Header;
    const HEADER: Header = Header::Completion;
}
