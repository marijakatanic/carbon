use crate::{database::prepare::BatchHolder, prepare::Extract};

use std::rc::Rc;

pub(crate) enum PrepareHandle {
    Batched {
        batch: Rc<BatchHolder>,
        index: usize,
    },
    Standalone(Extract),
}
