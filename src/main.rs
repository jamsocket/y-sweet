use clap::{Parser, Subcommand};
use std::net::IpAddr;

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
        #[clap(short, long, default_value = "8080")]
        port: u16,
        #[clap(short, long)]
        host: Option<IpAddr>,
        #[clap(short, long, default_value = "30")]
        checkpoint_freq_seconds: u32,
    },

    Dump,
}

fn main() {
    println!("Hello, world!");
}
