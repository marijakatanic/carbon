use crate::{crypto::Certificate, prepare::Prepare, signup::IdAssignment};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::{multi::Signature as MultiSignature, sign::Signature};

use zebra::vector::Vector;

#[derive(Serialize, Deserialize)]
pub(crate) enum PrepareRequest {
    Prepares(Vector<Prepare>),
    Signatures(MultiSignature, Vec<Option<Signature>>),
    Assignments(Vec<IdAssignment>),
    Witness(Certificate),
}
