FROM rust:1.73-slim

# Alternative approach:
# RUN cargo install wasm-pack

RUN apt-get update && apt-get install curl -y
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
