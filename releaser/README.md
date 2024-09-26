Releasing Y-Sweet
=================

**Note: this is only relevant to developers of Y-Sweet with publishing permissions. Regular users of Y-Sweet can ignore this crate.**

This Y-Sweet repo contains a number of packages in different languages, including the server in Rust and SDKs in JavaScript and Python.

Because new functionality may be exposed in the server API that needs to be supported in the SDKs, we keep version numbers across all packages in lockstep.

This releaser program automates the process of updating versions and publishing new releases.

## Bumping the version

To bump the version, first ensure that you are on `main` with no uncommitted changes. Then, run:

```
cargo run -- bump
```

This will:

1. Prompt you for a bump type (`patch`, `minor`, or `major`)
2. Update the version across all packages, including interdependencies.
3. Check out a new git branch named `release/X.Y.Z`
4. Push the branch and return a URL to the new PR.

From there, create a PR, go through the normal review process, and merge it before continuing.

## Creating a release

The release is created entirely through GitHub actions:

First, run the [release workflow](https://github.com/jamsocket/y-sweet/actions/workflows/release.yml) on GitHub Actions. The version number you enter **must** match the version number you bumped the version to.

If the action succeeds, it will create a new draft release on the [GitHub Releases](https://github.com/jamsocket/y-sweet/releases) page. Complete that draft release and publish it.

## Publishing packages

One of the packages (`y-sweet` on npm) refers to the binaries published in the last step.

To publish packages:

```
cargo run -- publish
```

This will:
- Check if the binaries have been published (this will fail if the previous step hasn't been completed)
- Compare the version of each package with the published version (on npm, crates.io, pypi)
- Publish each package that has a different published version
