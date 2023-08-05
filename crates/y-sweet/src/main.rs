#![doc = include_str!("../README.md")]

use crate::stores::filesystem::FileSystemStore;
use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use s3::Region;
use serde_json::json;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};
use stores::blobstore::S3Store;
use tracing::metadata::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use y_sweet_core::{auth::Authenticator, store::Store};

mod server;
mod stores;

#[derive(Parser)]
struct Opts {
    #[clap(subcommand)]
    subcmd: ServSubcommand,
}

#[derive(Subcommand)]
enum ServSubcommand {
    Serve {
        store: Option<String>,

        #[clap(long, default_value = "8080")]
        port: u16,
        #[clap(long)]
        host: Option<IpAddr>,
        #[clap(long, default_value = "10")]
        checkpoint_freq_seconds: u64,

        #[clap(long)]
        auth: Option<String>,

        #[clap(long)]
        use_https: bool,
    },

    GenAuth {
        #[clap(long)]
        json: bool,
    },
}

fn get_store_from_opts(store_path: &str) -> Result<Box<dyn Store>> {
    if store_path.starts_with("s3://") {
        let region = Region::UsEast1;

        let url = url::Url::parse(store_path)?;
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
        Ok(Box::new(FileSystemStore::new(PathBuf::from(store_path))?))
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

    match &opts.subcmd {
        ServSubcommand::Serve {
            port,
            host,
            checkpoint_freq_seconds,
            store,
            auth,
            use_https,
        } => {
            let auth = if let Some(auth) = auth {
                Some(Authenticator::new(auth)?)
            } else {
                tracing::warn!("No auth key set. Only use this for local development!");
                None
            };

            let addr = SocketAddr::new(
                host.unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
                *port,
            );

            let store = if let Some(store) = store {
                Some(get_store_from_opts(&store)?)
            } else {
                tracing::warn!("No store set. Documents will be stored in memory only.");
                None
            };

            let server = server::Server::new(
                store,
                std::time::Duration::from_secs(*checkpoint_freq_seconds),
                auth,
                *use_https,
            )
            .await?;

            let address = format!("http://{}:{}", addr.ip(), addr.port());
            tracing::info!(%address, "Listening");

            server.serve(&addr).await?;
        }
        ServSubcommand::GenAuth { json } => {
            let auth = Authenticator::gen_key()?;

            if *json {
                let result = json!({
                    "private_key": auth.private_key(),
                    "server_token": auth.server_token(),
                });

                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("Run y-sweet with the following option to require authentication:");
                println!();
                println!("   --auth {}", auth.private_key().bright_blue());
                println!();
                println!("Then, when interacting with y-sweet from your own server, pass the following server token:");
                println!();
                println!("   {}", auth.server_token().bright_purple());
                println!();
                println!("For example:");
                println!();
                println!("    // The token is hard-coded for simplicity of the example. Use a secret manager in production!");
                println!(
                    r#"    const params = {{"url": "http://127.0.0.1:8080", "token": "{}"}})"#,
                    auth.server_token().bright_purple()
                );
                println!("    const docInfo = createDoc(params)");
                println!("    const connectionKey = getClientToken(docInfo, {{}}, params)");
                println!();
                println!("Only use the server token on the server, do not expose the server token to clients.");
                println!("getConnectionKey() will return a derived token that clients can use to scoped to a specific document.");
            }
        }
    }

    Ok(())
}
