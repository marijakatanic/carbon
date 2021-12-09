use crate::{
    commit::Payload, database::Database, discovery::Client,
    processing::processor::commit::errors::ServeCommitError,
};

use doomstack::Top;

use talk::{
    crypto::{primitives::multi::Signature as MultiSignature, KeyChain},
    net::Session,
    sync::voidable::Voidable,
};

pub(in crate::processing::processor::commit) async fn validate_batch(
    _keychain: &KeyChain,
    _discovery: &Client,
    _database: &Voidable<Database>,
    _session: &mut Session,
    _payloads: &[Payload],
) -> Result<MultiSignature, Top<ServeCommitError>> {
    todo!()
}
