use clap::{Parser, Subcommand};
use server::Server;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};
use stores::FileSystemStore;

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

            server.serve().await.unwrap();
        }
        ServSubcommand::Dump => {
            todo!()
        }
    }
}
