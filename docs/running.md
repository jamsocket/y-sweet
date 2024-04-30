# Running a y-sweet server

The [quickstart](https://y-sweet.dev/quickstart) guide provides instructions for using our hosted y-sweet server, but if you prefer to host it on your own you have several options.

## Running a dev server

If you have `npm`, the fastest way to run a local server is with `npx`:

```bash
npx y-sweet@latest serve
```

This will download `y-sweet` if you do not already have it, and run it.

By default, `y-sweet serve` does not write data to disk. You can specify a directory to persist data to, like this:

```bash
npx y-sweet@latest serve /path/to/data
```

If the directory starts with `s3://`, `y-sweet` will treat it as an S3-compatible bucket path. In this case, `y-sweet` will pick up your local AWS credentials from the environment. If you do not have AWS credentials set up, you can set them up with `aws configure`.

## Running a Cloudflare Workers dev server

You can also run a local dev server based on the Cloudflare Workers runtime. This is only recommended for testing changes to the Cloudflare Workers code; if you just want to run a local server, the previous method is preferred.

Running the Cloudflare Worker requires cloning the repo and builing it from source:

```bash
git clone https://github.com/drifting-in-space/y-sweet.git
cd y-sweet/crates/y-sweet-worker
npm i
npm run dev
```

## Deploying to Cloudflare

To deploy to Cloudflare, use the `deploy` script:

```bash
git clone https://github.com/drifting-in-space/y-sweet.git
cd y-sweet/crates/y-sweet-worker
npm i
npm run deploy
```

See `y-sweet/crates/y-sweet-worker/wrangler.toml` for the Cloudflare resources it referenes. You will either need to create these resources or change the configuration to point to your own resources.

## Self-hosting in production

Docker images coming soon. If you're interested, [let us know](mailto:hi@driftingin.space).
