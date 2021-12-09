use crate::processing::{messages::CommitResponse, processor::commit::errors::ServeCommitError};

use doomstack::{here, ResultExt, Top};

use talk::net::Session;

pub(in crate::processing::processor::commit) async fn ping(
    mut session: Session,
) -> Result<(), Top<ServeCommitError>> {
    session
        .send(&CommitResponse::Pong)
        .await
        .pot(ServeCommitError::ConnectionError, here!())?;

    session.end();

    Ok(())
}
