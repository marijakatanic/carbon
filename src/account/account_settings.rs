#[derive(Debug, Clone)]
pub(crate) struct AccountSettings {
    pub supports_capacity: usize,
}

impl Default for AccountSettings {
    fn default() -> Self {
        AccountSettings {
            supports_capacity: 8,
        }
    }
}
