#[derive(Debug, Clone)]
pub(crate) struct SignupSettings {
    pub work_difficulty: u64,
}

impl Default for SignupSettings {
    fn default() -> Self {
        SignupSettings { work_difficulty: 8 }
    }
}
