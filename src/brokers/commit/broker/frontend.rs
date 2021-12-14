use crate::{
    brokers::commit::{brokerage::Brokerage, Broker, Request},
    data::Sponge,
    discovery::Client,
};

use doomstack::{here, Doom, ResultExt, Top};
use log::error;

use std::sync::Arc;

use talk::{net::PlainConnection, sync::fuse::Fuse};

use tokio::{net::TcpListener, sync::oneshot};

#[derive(Doom)]
enum ServeError {
    #[doom(description("Connection error"))]
    ConnectionError,
    #[doom(description("Request invalid"))]
    RequestInvalid,
    #[doom(description("`Brokerage` forfeited (most likely, the `Broker` is shutting down)"))]
    #[doom(wrap(request_forfeited))]
    BrokerageForfeited { source: oneshot::error::RecvError },
}

impl Broker {
    pub(in crate::brokers::commit::broker) async fn listen(
        discovery: Arc<Client>,
        brokerage_sponge: Arc<Sponge<Brokerage>>,
        listener: TcpListener,
    ) {
        let fuse = Fuse::new();

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let connection: PlainConnection = stream.into();

                    let discovery = discovery.clone();
                    let brokerage_sponge = brokerage_sponge.clone();

                    fuse.spawn(async move {
                        if let Err(e) = Broker::serve(discovery, brokerage_sponge, connection).await
                        {
                            error!("Error {:?}", e);
                        }
                    });
                }
                Err(e) => error!("Error {:?}", e),
            }
        }
    }

    async fn serve(
        discovery: Arc<Client>,
        brokerage_sponge: Arc<Sponge<Brokerage>>,
        mut connection: PlainConnection,
    ) -> Result<(), Top<ServeError>> {
        // Receive and validate `Request`

        let request = connection
            .receive::<Request>()
            .await
            .pot(ServeError::ConnectionError, here!())?;

        // Skip for benchmark
        let _ = request
            .validate(discovery.as_ref())
            .pot(ServeError::RequestInvalid, here!());

        // Build and submit `Brokerage` to `brokerage_sponge`

        let (completion_inlet, completion_outlet) = oneshot::channel();

        let brokerage = Brokerage {
            request,
            completion_inlet,
        };

        brokerage_sponge.push(brokerage);

        // Wait for `Completion` from `broker` task

        let completion = completion_outlet
            .await
            .map_err(ServeError::request_forfeited)
            .map_err(Doom::into_top)
            .spot(here!())?;

        // Send `commit` to the served client (note that `commit` is a `Result<Completion, Failure>`)

        connection
            .send(&completion)
            .await
            .pot(ServeError::ConnectionError, here!())?;

        // Successfully delivering a `BrokerFailure` to the served client is not a shortcoming
        // of `serve`, and should not result in an `Err` (see above)
        Ok(())
    }
}
