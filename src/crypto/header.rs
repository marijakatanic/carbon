use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i8)]
pub(crate) enum Header {
    RogueChallenge = 0,

    Install = 1,

    LatticeDecisions = 2,

    Resolution = 3,
    Resignation = 4,

    IdRequest = 5,
    IdAllocation = 6,
    IdAssignment = 7,

    Prepare = 8,
    PrepareReduction = 9,
    PrepareWitness = 10,
}
