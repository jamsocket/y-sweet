FROM rust:1.73-slim

RUN cargo install wasm-pack

RUN cargo install worker-build --version 0.0.10 --force

RUN rustup target add wasm32-unknown-unknown

RUN apt-get update && \
    apt-get install -y curl software-properties-common

RUN curl -sL https://deb.nodesource.com/setup_18.x | bash -

RUN apt-get install -y nodejs
