FROM rust:1.56-slim

RUN apt-get update && apt-get install curl -y

#RUN cargo install wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

