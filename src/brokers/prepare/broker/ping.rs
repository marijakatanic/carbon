use crate::{
    brokers::prepare::{Broker, PingBoard},
    processing::messages::{PrepareRequest, PrepareResponse},
};

use doomstack::{here, Doom, ResultExt};

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
    pub(in crate::brokers::prepare::broker) async fn ping(
        board: PingBoard,
        connector: Arc<SessionConnector>,
        replica: Identity,
        interval: Duration,
    ) {
        loop {
            let start = Instant::now();

            let ping = (async {
                let mut session = connector
                    .connect(replica)
                    .await
                    .pot(PingError::ConnectionFailed, here!())?;

                session
                    .send(&PrepareRequest::Ping)
                    .await
                    .pot(PingError::ConnectionError, here!())?;

                let response = session
                    .receive::<PrepareResponse>()
                    .await
                    .pot(PingError::ConnectionError, here!())?;

                match response {
                    PrepareResponse::Pong => Ok(start.elapsed()),
                    _ => PingError::UnexpectedResponse.fail().spot(here!()),
                }
            })
            .await;

            let ping = ping.unwrap_or(Duration::MAX);
            board.submit(replica, ping);

            time::sleep(interval).await;
        }
    }
}
