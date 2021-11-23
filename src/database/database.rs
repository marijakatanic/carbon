use crate::account::{Id, State};

use std::collections::HashMap;

use talk::crypto::KeyCard;

pub(crate) struct Database {
    keycards: HashMap<Id, KeyCard>,
    states: HashMap<Id, State>,
}
