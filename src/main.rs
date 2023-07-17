use crate::stores::filesystem::FileSystemStore;
use clap::{Parser, Subcommand};
use server::Server;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};
use tracing::metadata::LevelFilter;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

mod server;
mod stores;
mod sync_kv;

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
        #[clap(default_value = "30")]
        checkpoint_freq_seconds: u64,
    },

    Dump,
}

#[tokio::main]
async fn main() {
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

            let store = FileSystemStore::new(PathBuf::from(opts.store_path));

            let server = Server {
                store,
                addr,
                checkpoint_freq: std::time::Duration::from_secs(checkpoint_freq_seconds),
            };

            let address = format!("http://{}:{}", server.addr.ip(), server.addr.port());
            tracing::info!(%address, "Listening");

            server.serve().await.unwrap();
        }
        ServSubcommand::Dump => {
            todo!()
        }
    }
}
