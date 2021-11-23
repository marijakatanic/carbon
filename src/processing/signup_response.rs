use crate::signup::IdAllocation;

pub(crate) enum SignupResponse {
    IdAllocations(Vec<IdAllocation>),
}
