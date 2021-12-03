use crate::account::Id;

use zebra::database::Family;

pub(crate) struct Families {
    pub id: Family<Id>,
}
