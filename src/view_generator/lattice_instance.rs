use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub(in crate::view_generator) enum LatticeInstance {
    ViewLattice,
    SequenceLattice,
}
