use crate::{database::prepare::BatchHolder, prepare::Extract};

use std::sync::Arc;

pub(crate) enum PrepareHandle {
    Batched {
        batch: Arc<BatchHolder>,
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
