use crate::{
    account::{AccountSummary, Id},
    database::prepare::Advertisement as PrepareAdvertisement,
};

use zebra::database::{Database, Family};

pub(crate) struct Zebras {
    pub ids: Family<Id>,
    pub ids_to_account_summaries: Database<Id, AccountSummary>,
    pub ids_to_prepare_advertisements: Database<Id, PrepareAdvertisement>,
}

impl Zebras {
    pub fn new() -> Self {
        Zebras {
            ids: Family::new(),
            ids_to_account_summaries: Database::new(),
            ids_to_prepare_advertisements: Database::new(),
        }
    }
}
