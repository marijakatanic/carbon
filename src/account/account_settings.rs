#[derive(Debug, Clone)]
pub(crate) struct AccountSettings {
    // `initial_balance` should only be set to a non-zero value
    // for the sake of testing / benchmarking: use at own risk!
    pub initial_balance: u64,
    pub supports_capacity: usize,
}

impl Default for AccountSettings {
    fn default() -> Self {
        AccountSettings {
            initial_balance: 0,
            supports_capacity: 8,
        }
    }
}
