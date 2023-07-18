use crate::{api::serve_api, doc_service::DocService, stores::Store};
use std::{net::SocketAddr, sync::Arc, time::Duration};

pub struct Server {
    pub store: Box<dyn Store>,
    pub addr: SocketAddr,
    pub checkpoint_freq: Duration,
}

impl Server {
    pub async fn serve(self) -> Result<(), anyhow::Error> {
        let mut doc_server = DocService::new(Arc::new(self.store), self.checkpoint_freq).await?;
        let broadcast_group = doc_server.broadcast_group.take().unwrap();

        serve_api(&self.addr, broadcast_group).await
    }
}
