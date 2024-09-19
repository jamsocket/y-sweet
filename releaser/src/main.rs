use clap::{Parser, Subcommand};
use packages::PackageList;
use releaser::Releaser;
use semver::Version;

mod git;
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
    let mut packages = PackageList::new();
    packages.register_cargo_package("y-sweet", "crates/y-sweet");
    packages.register_cargo_package("y-sweet-core", "crates/y-sweet-core");
    packages.register_node_package("y-sweet", "js-pkg/server");
    packages.register_node_package("@y-sweet/sdk", "js-pkg/sdk");
    packages.register_node_package("@y-sweet/client", "js-pkg/client");
    packages.register_node_package("@y-sweet/react", "js-pkg/react");
    packages.register_python_package("y_sweet_sdk", "python");

    let releaser = Releaser::new(packages);
    let args = Args::parse();

    match args.command {
        Command::Bump { version } => {
            releaser.bump(version).unwrap();
        }
    }
}
