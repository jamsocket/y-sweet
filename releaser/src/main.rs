use packages::PackageList;

mod packages;
mod package_manager;

fn main() {
    let mut packages = PackageList::new();

    // packages.register_cargo_package("y-sweet", "crates/y-sweet");
    // packages.register_node_package("y-sweet", "js-pkg/server");
    // packages.register_node_package("@y-sweet/sdk", "js-pkg/sdk");
    // packages.register_node_package("@y-sweet/client", "js-pkg/client");
    // packages.register_node_package("@y-sweet/react", "js-pkg/react");
    packages.register_python_package("y_sweet_sdk", "python");

    for package in packages.iter() {
        println!("package: {}", package.name);

        let repo_version = package.get_repo_version().unwrap();
        println!("repo version: {}", repo_version);
        let public_version = package.get_public_version().unwrap();
        println!("public version: {}", public_version);
    }
    
    
}
