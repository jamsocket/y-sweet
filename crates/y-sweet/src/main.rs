use anyhow::Context;
use anyhow::Result;
use clap::{Parser, Subcommand};
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
use y_sweet::cli::{print_auth_message, print_server_url};
use y_sweet::stores::filesystem::FileSystemStore;
use y_sweet_core::{
    auth::Authenticator,
    store::{
        s3::{S3Config, S3Store},
        Store,
    },
};

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

        #[clap(long, default_value = "8080", env = "PORT")]
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

    ServeDoc {
        #[clap(long, default_value = "8080", env = "PORT")]
        port: u16,

        #[clap(long, env = "Y_SWEET_HOST")]
        host: Option<IpAddr>,

        #[clap(long, default_value = "10", env = "Y_SWEET_CHECKPOINT_FREQ_SECONDS")]
        checkpoint_freq_seconds: u64,
    },
}

const S3_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
const S3_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";
const S3_SESSION_TOKEN: &str = "AWS_SESSION_TOKEN";
const S3_REGION: &str = "AWS_REGION";
const S3_ENDPOINT: &str = "AWS_ENDPOINT_URL_S3";
fn parse_s3_config_from_env_and_args(bucket: String, prefix: String) -> anyhow::Result<S3Config> {
    Ok(S3Config {
        key: env::var(S3_ACCESS_KEY_ID)
            .map_err(|_| anyhow::anyhow!("{} env var not supplied", S3_ACCESS_KEY_ID))?,
        region: env::var(S3_REGION).unwrap_or_else(|_| DEFAULT_S3_REGION.to_string()),
        endpoint: env::var(S3_ENDPOINT).unwrap_or_else(|_| {
            format!(
                "https://s3.dualstack.{}.amazonaws.com",
                env::var(S3_REGION).unwrap_or_else(|_| DEFAULT_S3_REGION.to_string())
            )
        }),
        secret: env::var(S3_SECRET_ACCESS_KEY)
            .map_err(|_| anyhow::anyhow!("{} env var not supplied", S3_SECRET_ACCESS_KEY))?,
        token: env::var(S3_SESSION_TOKEN).ok(),
        bucket,
        bucket_prefix: Some(prefix),
    })
}

fn get_store_from_opts(store_path: &str) -> Result<Box<dyn Store>> {
    if store_path.starts_with("s3://") {
        let url = url::Url::parse(store_path)?;
        let bucket = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid S3 URL"))?
            .to_owned();
        let bucket_prefix = url.path().trim_start_matches('/').to_owned();
        let config = parse_s3_config_from_env_and_args(bucket, bucket_prefix)?;
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

            let server = y_sweet::server::Server::new(
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

            y_sweet::convert::convert(store, &buf, doc_id).await?;
        }
        ServSubcommand::Version => {
            println!("{}", VERSION);
        }
        ServSubcommand::ServeDoc {
            port,
            host,
            checkpoint_freq_seconds,
        } => {
            let doc_id = env::var("SESSION_BACKEND_KEY").expect("SESSION_BACKEND_KEY must be set");

            let bucket = env::var("STORAGE_BUCKET").ok();
            let prefix = env::var("STORAGE_PREFIX").ok();

            let store = match (bucket, prefix) {
                (Some(bucket), Some(prefix)) => {
                    let s3_config = parse_s3_config_from_env_and_args(bucket, prefix)
                        .context("Failed to parse S3 configuration")?;
                    let store = S3Store::new(s3_config);
                    let store: Box<dyn Store> = Box::new(store);
                    store
                        .init()
                        .await
                        .context("Failed to initialize S3 store")?;
                    Some(store)
                }
                (None, None) => {
                    tracing::warn!("No store set. Documents will be stored in memory only.");
                    None
                }
                _ => panic!("Invalid store configuration. Expected both STORAGE_BUCKET and STORAGE_PREFIX environment variables to be set."),
            };

            let cancellation_token = CancellationToken::new();
            let server = y_sweet::server::Server::new(
                store,
                std::time::Duration::from_secs(*checkpoint_freq_seconds),
                None, // No authenticator
                None, // No URL prefix
                cancellation_token.clone(),
            )
            .await?;

            // Load the one document we're operating with
            server
                .load_doc(&doc_id)
                .await
                .context("Failed to load document")?;

            let addr = SocketAddr::new(
                host.unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
                *port,
            );

            let server_handle = tokio::spawn(async move {
                server.serve_doc(&addr, false).await.unwrap();
            });

            tracing::info!("Listening on http://{}", addr);

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Received Ctrl+C, shutting down.");
                },
                _ = async {
                    #[cfg(unix)]
                    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                        Ok(mut signal) => signal.recv().await,
                        Err(e) => {
                            tracing::error!("Failed to install SIGTERM handler: {}", e);
                            std::future::pending::<Option<()>>().await
                        }
                    }

                    #[cfg(not(unix))]
                    std::future::pending::<Option<()>>().await
                } => {
                    tracing::info!("Received SIGTERM, shutting down.");
                }
            }

            cancellation_token.cancel();
            server_handle.await?;
            tracing::info!("Server shut down.");
        }
    }

    Ok(())
}
