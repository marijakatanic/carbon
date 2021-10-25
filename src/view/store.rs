use crate::view::{Change, View};

use lazy_static::lazy_static;

use std::collections::HashMap;
use std::sync::Mutex;

use zebra::database::Family;
use zebra::Commitment;

lazy_static! {
    pub(in crate::view) static ref FAMILY: Family<Change> = Family::new();
}

lazy_static! {
    pub(in crate::view) static ref VIEWS: Mutex<HashMap<Commitment, View>> =
        Mutex::new(HashMap::new());
}
