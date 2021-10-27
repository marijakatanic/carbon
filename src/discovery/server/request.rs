use crate::view::Install;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[repr(u8)]
pub(in crate::discovery::server) enum Request {
    Subscribe(u64),
    Publish(Install),
    KeepAlive,
}
