use talk::link::context::ListenDispatcherSettings;

#[derive(Debug, Clone, Default)]
pub(crate) struct ProcessorSettings {
    pub listen_dispatcher_settings: ListenDispatcherSettings,
    pub signup_settings: SignupSettings,
}

#[derive(Debug, Clone)]
pub(crate) struct SignupSettings {
    pub work_difficulty: u64,
    pub priority_attempts: usize,
}

impl Default for SignupSettings {
    fn default() -> Self {
        SignupSettings {
            work_difficulty: 8,
            priority_attempts: 32,
        }
    }
}
