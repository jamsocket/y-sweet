# y-serv

`y-serv` is a standalone yjs server with batteries included:
- Persistence to local disk, NFS, or S3-compatible blob storage (no database required!)
- PASETO-based authorization system.
- Built-in debug interface.
- History API.

## Usage

### Install the server

    cargo install --path=./

### Run a local server

You can run `y-serv` locally like this:

    y-serv ./data serve

This will use `./data` in the local directory as the document store and serve on localhost:8080.

### Create a new document

With the server running in the background, run:

    curl -X POST http://localhost:8080/doc/new

This will return something like:

    {"doc_id":"I_BtohX3OxHoPJJ4DUlBN"}

### Connect to a document:

    curl -X POST http://localhost:8080/doc/[doc_id from last step]/auth -d '{}' -H 'Content-Type: application/json'

