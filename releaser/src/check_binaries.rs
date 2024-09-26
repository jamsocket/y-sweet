use anyhow::Result;
use semver::Version;

const BINARY_URL_PREFIX: &str = "https://github.com/jamsocket/y-sweet/releases/download/";

/// The list of binaries to check for a given version.
/// This should match the artifacts produced by .github/workflows/release.yml
const BINARIES_TO_CHECK: &[&str] = &[
    "y-sweet-linux-x64.gz",
    "y-sweet-linux-arm64.gz",
    "y-sweet-macos-x64.gz",
    "y-sweet-macos-arm64.gz",
    "y-sweet-win-x64.exe.gz",
];

// Send a HEAD request to the given URL and return `true` if the status code is 200.
fn is_url_ok(url: &str) -> Result<bool> {
    let client = reqwest::blocking::Client::new();
    let res = client.head(url).send()?;
    Ok(res.status().is_success())
}

/// Check if all binaries for the given version have been released.
pub fn check_binaries(version: &Version) -> Result<bool> {
    let version_str = version.to_string();

    for binary in BINARIES_TO_CHECK {
        let url = format!("{BINARY_URL_PREFIX}v{version_str}/{binary}");
        if !is_url_ok(&url)? {
            println!("Binary {} is not found", binary);
            return Ok(false);
        }
    }

    Ok(true)
}
