use crate::{
    commit::Payload, database::Database, discovery::Client,
    processing::processor::commit::errors::ServeCommitError, view::View,
};

use doomstack::Top;

use talk::{crypto::KeyChain, net::Session, sync::voidable::Voidable};

pub(in crate::processing::processor::commit) async fn batch(
    _keychain: &KeyChain,
    _discovery: &Client,
    _view: &View,
    _database: &Voidable<Database>,
    _session: Session,
    _payloads: Vec<Payload>,
) -> Result<(), Top<ServeCommitError>> {
    todo!()
}
