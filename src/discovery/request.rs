use crate::view::Install;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[repr(u8)]
pub(in crate::discovery) enum Request {
    Subscribe(u64),
    FullSubscribe,
    Publish(Install),
    KeepAlive,
}
