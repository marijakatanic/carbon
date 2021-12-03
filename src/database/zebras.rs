use crate::account::Id;

use zebra::database::Family;

pub(crate) struct Zebras {
    pub ids: Family<Id>,
}

impl Zebras {
    pub fn new() -> Self {
        Zebras { ids: Family::new() }
    }
}
