use crate::stores::filesystem::FileSystemStore;
use anyhow::Result;
use clap::{Parser, Subcommand};
use s3::Region;
use server::Server;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};
use stores::{blobstore::S3Store, Store};
use tracing::metadata::LevelFilter;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

mod server;
mod stores;
mod sync_kv;
mod throttle;

#[derive(Parser)]
struct Opts {
    store_path: String,

    #[clap(subcommand)]
    subcmd: ServSubcommand,
}

#[derive(Subcommand)]
enum ServSubcommand {
    Serve {
        #[clap(default_value = "8000")]
        port: u16,
        host: Option<IpAddr>,
        #[clap(default_value = "10")]
        checkpoint_freq_seconds: u64,
    },

    Dump,
}

fn get_store_from_opts(opts: &Opts) -> Result<Box<dyn Store>> {
    if opts.store_path.starts_with("s3://") {
        let region = Region::UsEast1;

        let url = url::Url::parse(&opts.store_path)?;
        if url.scheme() != "s3" {
            return Err(anyhow::anyhow!("Invalid S3 URL"));
        }
        let bucket = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid S3 URL"))?
            .to_owned();
        let prefix = url.path().trim_start_matches('/').to_owned();

        let store = S3Store::new(region, bucket, prefix)?;
        Ok(Box::new(store))
    } else {
        Ok(Box::new(FileSystemStore::new(PathBuf::from(
            &opts.store_path,
        ))))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();

    match opts.subcmd {
        ServSubcommand::Serve {
            port,
            host,
            checkpoint_freq_seconds,
        } => {
            let addr = SocketAddr::new(
                host.unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
                port,
            );

            let store = get_store_from_opts(&opts)?;

            let server = Server {
                store,
                addr,
                checkpoint_freq: std::time::Duration::from_secs(checkpoint_freq_seconds),
            };

            let address = format!("http://{}:{}", server.addr.ip(), server.addr.port());
            tracing::info!(%address, "Listening");

            server.serve().await?;
        }
        ServSubcommand::Dump => {
            todo!()
        }
    }

    Ok(())
}
