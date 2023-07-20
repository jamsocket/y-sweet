use crate::auth::Authenticator;
use crate::stores::filesystem::FileSystemStore;
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use doc_service::DOC_NAME;
use lib0::any::Any;
use s3::Region;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};
use stores::{blobstore::S3Store, Store};
use tracing::metadata::LevelFilter;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
};
use yrs::{types::ToJson, Array, Doc, ReadTxn, Transact};
use yrs_kvstore::DocOps;

mod auth;
mod doc_service;
mod server;
mod stores;
mod sync_kv;
mod throttle;

#[derive(Parser)]
struct Opts {
    #[clap(subcommand)]
    subcmd: ServSubcommand,
}

#[derive(Subcommand)]
enum ServSubcommand {
    Serve {
        store_path: String,

        #[clap(long, default_value = "8080")]
        port: u16,
        #[clap(long)]
        host: Option<IpAddr>,
        #[clap(long, default_value = "10")]
        checkpoint_freq_seconds: u64,

        /// Bearer token required for document management API
        /// (not for direct client connections).
        #[clap(long)]
        bearer_token: Option<String>,

        #[clap(long)]
        paseto: Option<String>,
    },

    Dump {
        store_path: String,
        doc_id: String,
    },

    GenToken,
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
            bearer_token,
            store_path,
            paseto,
        } => {
            if bearer_token.is_none() {
                tracing::warn!("No bearer token set. Only use this for local development!");
            }

            let paseto = if let Some(paseto) = paseto {
                Some(Authenticator::new(paseto)?)
            } else {
                None
            };

            let addr = SocketAddr::new(
                host.unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
                *port,
            );

            let store = get_store_from_opts(&store_path)?;

            let server = server::Server::new(
                store,
                std::time::Duration::from_secs(*checkpoint_freq_seconds),
                bearer_token.clone(),
                paseto,
            )
            .await?;

            let address = format!("http://{}:{}", addr.ip(), addr.port());
            tracing::info!(%address, "Listening");

            server.serve(&addr).await?;
        }
        ServSubcommand::Dump { doc_id, store_path } => {
            let store = get_store_from_opts(&store_path)?;
            let sync_kv = sync_kv::SyncKv::new(Arc::new(store), doc_id, || {}).await?;
            let doc = Doc::new();

            {
                let mut txn = doc.transact_mut();
                sync_kv
                    .load_doc(DOC_NAME, &mut txn)
                    .map_err(|e| anyhow!("Error loading doc: {:?}", e))?;

                let root_keys = txn.root_keys();

                let mut map: HashMap<String, Any> = HashMap::new();

                for key in root_keys {
                    let value = txn.get_array(&key).expect("Failed to get array");
                    if value.len(&txn) > 0 {
                        map.insert(key.to_string(), value.to_json(&txn));
                        continue;
                    }

                    let value = txn.get_map(&key).expect("Failed to get map");
                    map.insert(key.to_string(), value.to_json(&txn));
                }

                let result = Any::Map(Box::new(map));

                dump_object(&result);
            }
        }
        ServSubcommand::GenToken => {
            let key = Authenticator::gen_key()?;

            println!("Run y-serve with the following option to enable PASETO tokens:");
            println!();
            println!("   --paseto {}", key);
        }
    }

    Ok(())
}

fn dump_object_inner(result: &Any, indent: usize) {
    let indent_str = "  ".repeat(indent);

    match result {
        Any::Map(map) => {
            println!("{{");
            for (key, value) in map.iter() {
                print!("  {}{}: ", indent_str, key);
                dump_object_inner(value, indent + 1);
            }
            println!("{}}}", indent_str);
        }
        Any::Array(array) => {
            println!("[");
            for value in array.iter() {
                print!("{}  ", indent_str);
                dump_object_inner(value, indent + 1);
            }
            println!("{}]", indent_str);
        }
        Any::String(string) => {
            println!("\"{}\"", string.replace("\"", "\\\""));
        }
        Any::Number(number) => {
            println!("{}", number);
        }
        Any::Bool(boolean) => {
            println!("{}", boolean);
        }
        Any::Null => {
            println!("null");
        }
        Any::Undefined => todo!(),
        Any::BigInt(_) => todo!(),
        Any::Buffer(_) => todo!(),
    }
}

fn dump_object(result: &Any) {
    dump_object_inner(result, 0);
}
