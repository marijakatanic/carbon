use crate::view::Change;

use lazy_static::lazy_static;

use zebra::database::Family;

lazy_static! {
    pub(in crate::view) static ref STORE: Family<Change> = Family::new();
}
