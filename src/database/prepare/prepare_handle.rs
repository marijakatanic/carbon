use crate::{database::prepare::BatchHolder, prepare::Extract};

use std::rc::Rc;

pub(crate) enum PrepareHandle {
    Batched {
        batch: Rc<BatchHolder>,
        index: usize,
    },
    Standalone(Extract),
}

impl PrepareHandle {
    pub fn extract(&self) -> Extract {
        match self {
            PrepareHandle::Batched { batch, index } => batch.extract(*index),
            PrepareHandle::Standalone(extract) => extract.clone(),
        }
    }
}
