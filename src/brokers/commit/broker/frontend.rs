use crate::{
    brokers::commit::{brokerage::Brokerage, Broker},
    data::Sponge,
    discovery::Client,
};

use doomstack::{Doom, Top};

use std::sync::Arc;

use talk::{net::PlainConnection, sync::fuse::Fuse};

use tokio::net::TcpListener;

#[derive(Doom)]
enum ServeError {
    #[doom(description("Connection error"))]
    ConnectionError,
}

impl Broker {
    pub(in crate::brokers::commit::broker) async fn listen(
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
        _discovery: Arc<Client>,
        _brokerage_sponge: Arc<Sponge<Brokerage>>,
        _connection: PlainConnection,
    ) -> Result<(), Top<ServeError>> {
        todo!()
    }
}
