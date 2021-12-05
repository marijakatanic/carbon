use crate::{
    brokers::prepare::{
        broker::{Brokerage, Reduction},
        Broker, Failure, Inclusion, Request,
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
    #[doom(description("Root shard invalid"))]
    RootShardInvalid,
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
                    let _ = Broker::serve(connection, discovery, brokerage_sponge).await;
                });
            }
        }
    }

    async fn serve(
        mut connection: PlainConnection,
        discovery: Arc<Client>,
        brokerage_sponge: Arc<Sponge<Brokerage>>,
    ) -> Result<(), Top<ServeError>> {
        let request = connection
            .receive::<Request>()
            .await
            .pot(ServeError::ConnectionError, here!())?;

        request
            .validate(discovery.as_ref())
            .pot(ServeError::RequestInvalid, here!())?;

        let keycard = request.keycard().clone();

        let (reduction_inlet, reduction_outlet) = oneshot::channel();

        let brokerage = Brokerage {
            request,
            reduction_inlet,
        };

        brokerage_sponge.push(brokerage);

        let reduction = reduction_outlet
            .await
            .map_err(ServeError::request_forfeited)
            .map_err(Doom::into_top)
            .spot(here!())?;

        if let Err(failure) = reduction {
            connection
                .send::<Result<Inclusion, Failure>>(&Err(failure))
                .await
                .pot(ServeError::ConnectionError, here!())?;

            return Ok(());
        }

        let Reduction {
            index,
            inclusion,
            reduction_sponge,
        } = reduction.unwrap();

        let root = inclusion.root();

        connection
            .send::<Result<Inclusion, Failure>>(&Ok(inclusion))
            .await
            .pot(ServeError::ConnectionError, here!())?;

        let reduction_shard = connection
            .receive::<MultiSignature>()
            .await
            .pot(ServeError::ConnectionError, here!())?;

        reduction_shard
            .verify([&keycard], &ReductionStatement::new(root))
            .pot(ServeError::RootShardInvalid, here!())?;

        let _ = reduction_sponge.push((index, reduction_shard));

        // TODO: Wait for and forward outcome to client
        todo!()
    }
}
