# Running a Y-Sweet server

The [quickstart](https://docs.jamsocket.com/y-sweet/quickstart) guide provides instructions for using hosted Y-Sweet on Jamsocket, but if you prefer to host it on your own you have several options.

## Running a dev server

If you have `npm`, the fastest way to run a local server is with `npx`:

```bash
npx y-sweet@latest serve
```

This will download the Y-Sweet server if you do not already have it, and run it.

By default, `y-sweet serve` does not write data to disk. You can specify a directory to persist data to, like this:

```bash
npx y-sweet@latest serve /path/to/data
```

If the directory starts with `s3://`, Y-Sweet will treat it as an S3-compatible bucket path. In this case, Y-Sweet will pick up your local AWS credentials from the environment. If you do not have AWS credentials set up, you can set them up with `aws configure`.

## Deploying to Jamsocket

Run the Y-Sweet server on [Jamsocket's session backends](https://jamsocket.com/y-sweet). Check out the [quickstart](https://docs.jamsocket.com/y-sweet/quickstart) guide to get up and running in just a few minutes.

## Docker Image

The latest Docker image is available as `ghcr.io/jamsocket/y-sweet:latest`. You can find a [list of images here](https://github.com/jamsocket/y-sweet/pkgs/container/y-sweet).
