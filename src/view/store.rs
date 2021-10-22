use crate::view::Change;

use lazy_static::lazy_static;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use talk::crypto::primitives::sign::PublicKey;

use zebra::database::{Collection, Family};
use zebra::Commitment;

lazy_static! {
    pub(in crate::view) static ref FAMILY: Family<Change> = Family::new();
}

lazy_static! {
    pub(in crate::view) static ref CHANGES: Arc<Mutex<HashMap<Commitment, Collection<Change>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

lazy_static! {
    pub(in crate::view) static ref MEMBERS: Arc<Mutex<HashMap<Commitment, Vec<PublicKey>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}
