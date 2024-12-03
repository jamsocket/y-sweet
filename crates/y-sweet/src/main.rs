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
use tokio::net::TcpListener;
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
const S3_USE_PATH_STYLE: &str = "AWS_S3_USE_PATH_STYLE";
fn parse_s3_config_from_env_and_args(
    bucket: String,
    prefix: Option<String>,
) -> anyhow::Result<S3Config> {
    let use_path_style = env::var(S3_USE_PATH_STYLE).ok();
    let path_style = if let Some(use_path_style) = use_path_style {
        if use_path_style.to_lowercase() == "true" {
            true
        } else if use_path_style.to_lowercase() == "false" || use_path_style.is_empty() {
            false
        } else {
            anyhow::bail!(
                "If AWS_S3_USE_PATH_STYLE is set, it must be either \"true\" or \"false\""
            )
        }
    } else {
        false
    };

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
        bucket_prefix: prefix,
        // If the endpoint is overridden, we assume that the user wants path-style URLs.
        path_style,
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
        let bucket_prefix = (!bucket_prefix.is_empty()).then_some(bucket_prefix); // "" => None
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

            let listener = TcpListener::bind(addr).await?;
            let addr = listener.local_addr()?;

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
                true,
            )
            .await?;

            let prod = *prod;
            let handle = tokio::spawn(async move {
                server.serve(listener, prod).await.unwrap();
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

            let store = if let Ok(bucket) = env::var("STORAGE_BUCKET") {
                let prefix = if let Ok(prefix) = env::var("STORAGE_PREFIX") {
                    // If the prefix is set, it should contain the document ID as its last '/'-separated part.
                    // We want to pop that, because we will add it back when accessing the doc.
                    let mut parts: Vec<&str> = prefix.split('/').collect();
                    if let Some(last) = parts.pop() {
                        if last != doc_id {
                            anyhow::bail!("STORAGE_PREFIX must end with the document ID. Found: {} Expected: {}", last, doc_id);
                        }

                        let prefix = parts.join("/");

                        Some(prefix)
                    } else {
                        // As far as y-sweet is concerned, `STORAGE_BUCKET` = "" is equivalent to `STORAGE_BUCKET` not being set.
                        None
                    }
                } else {
                    None
                };

                let s3_config = parse_s3_config_from_env_and_args(bucket, prefix)?;
                let store = S3Store::new(s3_config);
                let store: Box<dyn Store> = Box::new(store);
                store.init().await?;
                Some(store)
            } else {
                if env::var("STORAGE_PREFIX").is_ok() {
                    anyhow::bail!("If STORAGE_PREFIX is set, STORAGE_BUCKET must also be set.");
                }

                None
            };

            let cancellation_token = CancellationToken::new();
            let server = y_sweet::server::Server::new(
                store,
                std::time::Duration::from_secs(*checkpoint_freq_seconds),
                None, // No authenticator
                None, // No URL prefix
                cancellation_token.clone(),
                false,
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

            let listener = TcpListener::bind(addr).await?;
            let addr = listener.local_addr()?;

            tokio::spawn(async move {
                server.serve_doc(listener, false).await.unwrap();
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
            tracing::info!("Server shut down.");
        }
    }

    Ok(())
}
