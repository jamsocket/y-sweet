use check_binaries::check_binaries;
use clap::{Parser, Subcommand};
use packages::PackageList;
use releaser::Releaser;
use semver::Version;

mod check_binaries;
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

    Publish,
}

fn main() {
    let mut packages = PackageList::new();
    packages.register_cargo_package("y-sweet", "crates/y-sweet");
    packages.register_cargo_package("y-sweet-core", "crates/y-sweet-core");
    let server_pkg = packages.register_node_package("y-sweet", "js-pkg/server");
    packages.register_node_package("@y-sweet/sdk", "js-pkg/sdk");
    packages.register_node_package("@y-sweet/client", "js-pkg/client");
    packages.register_node_package("@y-sweet/react", "js-pkg/react");
    packages.register_node_package("y-sweet-tests", "tests");
    packages.register_python_package("y_sweet_sdk", "python");

    let releaser = Releaser::new(packages);
    let args = Args::parse();

    match args.command {
        Command::Bump { version } => {
            releaser.bump(version).unwrap();
        }
        Command::Publish => {
            let server_pkg_version = server_pkg.get_published_version().unwrap();
            println!("Server package version: {}", server_pkg_version);
            check_binaries(&server_pkg_version).unwrap();
        }
    }
}
