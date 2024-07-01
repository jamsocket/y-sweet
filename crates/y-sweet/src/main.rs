#![doc = include_str!("../README.md")]

use crate::stores::filesystem::FileSystemStore;
use anyhow::Result;
use clap::{Parser, Subcommand};
use cli::{print_auth_message, print_server_url};
use serde_json::json;
use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};
use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;
use tracing::metadata::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use url::Url;
use y_sweet_core::{
    auth::Authenticator,
    store::{
        s3::{S3Config, S3Store},
        Store,
    },
};

mod cli;
mod convert;
mod server;
mod stores;

const DEFAULT_S3_REGION: &str = "us-east-1";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
struct Opts {
    #[clap(subcommand)]
    subcmd: ServSubcommand,
}

#[derive(Subcommand)]
enum ServSubcommand {
    Serve {
        #[clap(env = "Y_SWEET_STORE")]
        store: Option<String>,

        #[clap(long, default_value = "8080", env = "Y_SWEET_PORT")]
        port: u16,
        #[clap(long, env = "Y_SWEET_HOST")]
        host: Option<IpAddr>,
        #[clap(long, default_value = "10", env = "Y_SWEET_CHECKPOINT_FREQ_SECONDS")]
        checkpoint_freq_seconds: u64,

        #[clap(long, env = "Y_SWEET_AUTH")]
        auth: Option<String>,

        #[clap(long, env = "Y_SWEET_URL_PREFIX")]
        url_prefix: Option<Url>,

        #[clap(long)]
        prod: bool,
    },

    GenAuth {
        #[clap(long)]
        json: bool,
    },

    /// Convert from a YDoc v1 update format to a .ysweet file.
    /// The YDoc update should be passed in via stdin.
    ConvertFromUpdate {
        /// The store to write the document to.
        #[clap(env = "Y_SWEET_STORE")]
        store: String,

        /// The ID of the document to write.
        doc_id: String,
    },

    Version,
}

const S3_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
const S3_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";
const S3_REGION: &str = "AWS_REGION";
const S3_ENDPOINT: &str = "AWS_ENDPOINT_URL_S3";
const S3_BUCKET_PREFIX: &str = "S3_BUCKET_PREFIX";
const S3_BUCKET_NAME: &str = "S3_BUCKET_NAME";
fn parse_s3_config_from_env_and_args(
    bucket: Option<String>,
    prefix: Option<String>,
) -> anyhow::Result<S3Config> {
    let region = env::var(S3_REGION).unwrap_or(DEFAULT_S3_REGION.into());

    //default to using aws
    let endpoint = env::var(S3_ENDPOINT).map_or_else(
        |_| format!("https://s3.dualstack.{}.amazonaws.com", region),
        |s| s.to_string(),
    );

    Ok(S3Config {
        key: env::var(S3_ACCESS_KEY_ID)
            .map_err(|_| anyhow::anyhow!("AWS_ACCESS_KEY_ID env var not supplied"))?
            .to_string(),
        region,
        endpoint,
        secret: env::var(S3_SECRET_ACCESS_KEY)
            .map_err(|_| anyhow::anyhow!("AWS_SECRET_ACCESS_KEY env var not supplied"))?
            .to_string(),
        bucket: {
            if let Ok(bucket_name) = env::var(S3_BUCKET_NAME) {
                bucket_name
            } else {
                bucket.ok_or_else(|| anyhow::anyhow!("S3_BUCKET_NAME env var not supplied"))?
            }
        },
        bucket_prefix: {
            if let Ok(bucket_prefix) = env::var(S3_BUCKET_PREFIX) {
                Some(bucket_prefix)
            } else {
                prefix
            }
        },
    })
}

fn get_store_from_opts(store_path: &str) -> Result<Box<dyn Store>> {
    if store_path.starts_with("s3://") {
        let url = url::Url::parse(store_path)?;
        let bucket = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid S3 URL"))?
            .to_owned();
        let bucket_prefix = Some(url.path().trim_start_matches('/').to_owned());
        let config = parse_s3_config_from_env_and_args(Some(bucket), bucket_prefix)?;
        let store = S3Store::new(config);
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
            url_prefix,
            prod,
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
                let store = get_store_from_opts(store)?;
                store.init().await?;
                Some(store)
            } else {
                tracing::warn!("No store set. Documents will be stored in memory only.");
                None
            };

            if !prod {
                print_server_url(auth.as_ref(), url_prefix.as_ref(), addr);
            }

            let token = CancellationToken::new();

            let server = server::Server::new(
                store,
                std::time::Duration::from_secs(*checkpoint_freq_seconds),
                auth,
                url_prefix.clone(),
                token.clone(),
            )
            .await?;

            let prod = *prod;
            let handle = tokio::spawn(async move {
                server.serve(&addr, prod).await.unwrap();
            });

            tracing::info!("Listening on ws://{}", addr);

            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install CTRL+C signal handler");

            tracing::info!("Shutting down.");
            token.cancel();

            handle.await?;
            tracing::info!("Server shut down.");
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
                print_auth_message(&auth);
            }
        }
        ServSubcommand::ConvertFromUpdate { store, doc_id } => {
            let store = get_store_from_opts(store)?;
            store.init().await?;

            let mut stdin = tokio::io::stdin();
            let mut buf = Vec::new();
            stdin.read_to_end(&mut buf).await?;

            convert::convert(store, &buf, doc_id).await?;
        }
        ServSubcommand::Version => {
            println!("{}", VERSION);
        }
    }

    Ok(())
}
