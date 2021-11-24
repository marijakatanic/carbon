use crate::view::{Change, View};

use lazy_static::lazy_static;

use std::{collections::HashMap, sync::Mutex};

use talk::crypto::primitives::hash::Hash;

use zebra::database::Family;

lazy_static! {
    pub(in crate::view) static ref FAMILY: Family<Change> = Family::new();
}

lazy_static! {
    pub(in crate::view) static ref VIEWS: Mutex<HashMap<Hash, View>> = Mutex::new(HashMap::new());
}
