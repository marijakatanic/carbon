use talk::link::context::ListenDispatcherSettings;

#[derive(Debug, Clone, Default)]
pub(crate) struct ProcessorSettings {
    listen_dispatcher_settings: ListenDispatcherSettings,
    signup: SignupSettings,
}

#[derive(Debug, Clone)]
pub(crate) struct SignupSettings {
    work_difficulty: u64,
    priority_attempts: usize,
}

impl Default for SignupSettings {
    fn default() -> Self {
        SignupSettings {
            work_difficulty: 8,
            priority_attempts: 32,
        }
    }
}
