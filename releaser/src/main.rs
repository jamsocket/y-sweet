use clap::{Parser, Subcommand};
use releaser::Releaser;
use semver::Version;

mod package_manager;
mod packages;
mod releaser;
mod util;

#[derive(Parser)]
#[clap(version = "0.1.0", author = "Y-sweet")]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Bump {
        /// The version to bump to.
        #[clap(long)]
        version: Option<Version>,
    },
}

fn main() {
    let releaser = Releaser::new();

    let args = Args::parse();

    match args.command {
        Command::Bump { version } => {
            releaser.bump(version).unwrap();
        }
    }
}
