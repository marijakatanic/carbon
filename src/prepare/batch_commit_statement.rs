use crate::{account::Id, crypto::Header};

use serde::Serialize;

use std::collections::BTreeSet;

use talk::crypto::{primitives::hash::Hash, Statement};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BatchCommitStatement {
    view: Hash,
    root: Hash,
    exceptions: BTreeSet<Id>,
}

impl BatchCommitStatement {
    pub fn new(view: Hash, root: Hash, exceptions: BTreeSet<Id>) -> Self {
        BatchCommitStatement {
            view,
            root,
            exceptions,
        }
    }
}

impl Statement for BatchCommitStatement {
    type Header = Header;
    const HEADER: Header = Header::Commit;
}
