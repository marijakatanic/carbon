use crate::{
    brokers::commit::Broker,
    data::PingBoard,
    processing::messages::{CommitRequest, CommitResponse},
};

use doomstack::{here, Doom, ResultExt, Top};

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use talk::{crypto::Identity, net::SessionConnector};

use tokio::time;

#[derive(Doom)]
enum PingError {
    #[doom(description("Connection failed"))]
    ConnectionFailed,
    #[doom(description("Connection error"))]
    ConnectionError,
    #[doom(description("Unexpected response"))]
    UnexpectedResponse,
}

impl Broker {
    pub(in crate::brokers::commit::broker) async fn ping(
        board: PingBoard,
        connector: Arc<SessionConnector>,
        replica: Identity,
    ) {
        loop {
            let start = Instant::now();

            let ping: Result<Duration, Top<PingError>> = (async {
                let mut session = connector
                    .connect(replica)
                    .await
                    .pot(PingError::ConnectionFailed, here!())?;

                session
                    .send(&CommitRequest::Ping)
                    .await
                    .pot(PingError::ConnectionError, here!())?;

                let response = session
                    .receive::<CommitResponse>()
                    .await
                    .pot(PingError::ConnectionError, here!())?;

                match response {
                    CommitResponse::Pong => Ok(()),
                    _ => PingError::UnexpectedResponse.fail().spot(here!()),
                }?;

                Ok(start.elapsed())
            })
            .await;

            // If pinging was impossible, assign `replica` the highest
            // possible score (replicas whose pings failed are at the
            // end of the `PingBoard`)
            let ping = ping.unwrap_or(Duration::MAX);
            board.submit(replica, ping);

            time::sleep(Duration::from_secs(60)).await; // TODO: Add settings
        }
    }
}
