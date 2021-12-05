use crate::{brokers::prepare::Inclusion, data::Sponge};

use std::sync::Arc;

use talk::crypto::primitives::multi::Signature as MultiSignature;

pub(in crate::brokers::prepare) struct Reduction {
    pub index: usize,
    pub inclusion: Inclusion,
    pub reduction_sponge: Arc<Sponge<(usize, MultiSignature)>>,
}
