use std::sync::Arc;
use std::time::Duration;

use talk::time::sleep_schedules::CappedExponential;
use talk::time::SleepSchedule;

#[derive(Debug, Clone)]
pub(crate) struct ClientSettings {
    pub keepalive_interval: Duration,
    pub retry_schedule: Arc<dyn SleepSchedule>,
}

impl Default for ClientSettings {
    fn default() -> Self {
        ClientSettings {
            keepalive_interval: Duration::from_secs(10),
            retry_schedule: Arc::new(CappedExponential::new(
                Duration::from_secs(5),
                2.,
                Duration::from_secs(300),
            )),
        }
    }
}
