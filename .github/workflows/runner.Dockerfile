FROM rust:1.73-slim

RUN cargo install wasm-pack

RUN cargo install worker-build --version 0.0.10 --force

RUN rustup target add wasm32-unknown-unknown
