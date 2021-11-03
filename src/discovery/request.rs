use crate::view::Install;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[repr(u8)]
pub(in crate::discovery) enum Request {
    LightSubscribe(u64),
    FullSubscribe,
    Publish(Install),
    KeepAlive,
}
