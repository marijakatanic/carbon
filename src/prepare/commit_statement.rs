use crate::{account::Id, crypto::Header};

use serde::Serialize;

use std::collections::BTreeSet;

use talk::crypto::{primitives::hash::Hash, Statement};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CommitStatement {
    view: Hash,
    root: Hash,
    exceptions: BTreeSet<Id>,
}

impl CommitStatement {
    pub fn new<I>(view: Hash, root: Hash, exceptions: I) -> Self
    where
        I: IntoIterator<Item = Id>,
    {
        let exceptions = exceptions.into_iter().collect();

        CommitStatement {
            view,
            root,
            exceptions,
        }
    }
}

impl Statement for CommitStatement {
    type Header = Header;
    const HEADER: Header = Header::Commit;
}
