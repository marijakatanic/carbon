use crate::processing::{messages::PrepareResponse, processor::prepare::errors::ServePrepareError};

use doomstack::{here, ResultExt, Top};

use talk::net::Session;

pub(in crate::processing::processor::prepare) async fn ping(
    mut session: Session,
) -> Result<(), Top<ServePrepareError>> {
    session
        .send(&PrepareResponse::Pong)
        .await
        .pot(ServePrepareError::ConnectionError, here!())?;

    session.end();

    Ok(())
}
