use crate::stores::filesystem::FileSystemStore;
use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use doc_service::DOC_NAME;
use lib0::any::Any;
use s3::Region;
use yrs::{Doc, Transact, ReadTxn, types::ToJson, Array};
use yrs_kvstore::DocOps;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf, sync::Arc, collections::HashMap,
};
use stores::{blobstore::S3Store, Store};
use tracing::metadata::LevelFilter;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

mod doc_service;
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
    },

    Dump {
        doc_id: String,
    },
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
        ))?))
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
        } => {
            if bearer_token.is_none() {
                tracing::warn!("No bearer token set. Only use this for local development!");
            }

            let addr = SocketAddr::new(
                host.unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
                *port,
            );

            let store = get_store_from_opts(&opts)?;

            let server = server::Server::new(
                store,
                std::time::Duration::from_secs(*checkpoint_freq_seconds),
                bearer_token.clone(),
            )
            .await?;

            let address = format!("http://{}:{}", addr.ip(), addr.port());
            tracing::info!(%address, "Listening");

            server.serve(&addr).await?;
        }
        ServSubcommand::Dump { doc_id } => {
            let store = get_store_from_opts(&opts)?;
            let sync_kv = sync_kv::SyncKv::new(Arc::new(store), doc_id, || {}).await?;
            let doc = Doc::new();
            
            {
                let mut txn = doc.transact_mut();
                sync_kv.load_doc(DOC_NAME, &mut txn).map_err(|e| anyhow!("Error loading doc: {:?}", e))?;

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
