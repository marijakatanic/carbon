use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub(in crate::view_generator) enum LatticeInstance {
    ViewLattice = 0,
    SequenceLattice = 1,
}
