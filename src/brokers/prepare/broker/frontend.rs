use crate::{
    brokers::prepare::{
        broker::{Brokerage, Reduction},
        Broker, BrokerFailure, Inclusion, Request,
    },
    data::Sponge,
    discovery::Client,
    prepare::ReductionStatement,
};

use doomstack::{here, Doom, ResultExt, Top};

use std::sync::Arc;

use talk::{
    crypto::primitives::multi::Signature as MultiSignature, net::PlainConnection, sync::fuse::Fuse,
};

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
    #[doom(description("Reduction shard invalid"))]
    ReductionShardInvalid,
}

impl Broker {
    pub(in crate::brokers::prepare::broker) async fn listen(
        discovery: Arc<Client>,
        brokerage_sponge: Arc<Sponge<Brokerage>>,
        listener: TcpListener,
    ) {
        let fuse = Fuse::new();

        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let connection: PlainConnection = stream.into();

                let discovery = discovery.clone();
                let brokerage_sponge = brokerage_sponge.clone();

                fuse.spawn(async move {
                    let _ = Broker::serve(discovery, brokerage_sponge, connection).await;
                });
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

        request
            .validate(discovery.as_ref())
            .pot(ServeError::RequestInvalid, here!())?;

        // Build and submit `Brokerage` to `brokerage_sponge`

        let keycard = request.keycard().clone(); // Needed to later verify the client's reduction shard

        let (reduction_inlet, reduction_outlet) = oneshot::channel();
        let (commit_inlet, commit_outlet) = oneshot::channel();

        let brokerage = Brokerage {
            request,
            reduction_inlet,
            commit_inlet,
        };

        brokerage_sponge.push(brokerage);

        // Wait for `Reduction` from `broker` task

        let reduction = reduction_outlet
            .await
            .map_err(ServeError::request_forfeited)
            .map_err(Doom::into_top)
            .spot(here!())?;

        // If `reduction` is `Err`, forward `BrokerFailure` to the served client,
        // otherwise explode `reduction`'s fields

        let Reduction {
            index,
            inclusion,
            reduction_sponge,
        } = match reduction {
            Ok(reduction) => reduction,
            Err(failure) => {
                connection
                    .send::<Result<Inclusion, BrokerFailure>>(&Err(failure))
                    .await
                    .pot(ServeError::ConnectionError, here!())?;

                // Successfully delivering a `BrokerFailure` to the served client is not a
                // shortcoming of `serve`, and should not result in an `Err`
                return Ok(());
            }
        };

        let root = inclusion.root(); // Needed to later verify the client's reduction shard

        // Trade `inclusion` for a (valid) reduction shard

        connection
            .send::<Result<Inclusion, BrokerFailure>>(&Ok(inclusion))
            .await
            .pot(ServeError::ConnectionError, here!())?;

        let reduction_shard = connection
            .receive::<MultiSignature>()
            .await
            .pot(ServeError::ConnectionError, here!())?;

        reduction_shard
            .verify([&keycard], &ReductionStatement::new(root))
            .pot(ServeError::ReductionShardInvalid, here!())?;

        // Submit `reduction_shard` to `reduction_sponge`

        let _ = reduction_sponge.push((index, reduction_shard));

        // Wait for `BatchCommit` from `broker` task

        let commit = commit_outlet
            .await
            .map_err(ServeError::request_forfeited)
            .map_err(Doom::into_top)
            .spot(here!())?;

        // Send `commit` to the served client (note that `commit` is a `Result<BatchCommit, Failure>`)

        connection
            .send(&commit)
            .await
            .pot(ServeError::ConnectionError, here!())?;

        // Successfully delivering a `BrokerFailure` to the served client is not a shortcoming
        // of `serve`, and should not result in an `Err` (see above)
        Ok(())
    }
}
