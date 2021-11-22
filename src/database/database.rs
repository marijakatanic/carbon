use crate::account::{Id, State};

use std::collections::HashMap;

pub(crate) struct Database {
    states: HashMap<Id, State>,
}
