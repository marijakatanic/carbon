use crate::signup::SignupSettings;

use talk::link::context::ListenDispatcherSettings;

#[derive(Debug, Clone, Default)]
pub(crate) struct ProcessorSettings {
    pub listen_dispatcher_settings: ListenDispatcherSettings,
    pub signup: Signup,
}

#[derive(Debug, Clone)]
pub(crate) struct Signup {
    pub signup_settings: SignupSettings,
    pub priority_attempts: usize,
}

impl Default for Signup {
    fn default() -> Self {
        Signup {
            signup_settings: SignupSettings::default(),
            priority_attempts: 32,
        }
    }
}
