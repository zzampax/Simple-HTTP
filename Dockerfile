FROM rust:latest

RUN mkdir -p /root/simple-http
WORKDIR /root/simple-http

COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src
COPY ./public ./public

RUN cargo build --release
CMD [ "cargo", "run", "--release" ]