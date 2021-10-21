use crate::view::Change;

use lazy_static::lazy_static;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use zebra::database::{Collection, Family};
use zebra::Commitment;

lazy_static! {
    pub(in crate::view) static ref FAMILY: Family<Change> = Family::new();
    pub(in crate::view) static ref CHANGES: Arc<Mutex<HashMap<Commitment, Collection<Change>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}
