# adapted from the following sources:
# - https://github.com/linux-china/axum-demo/blob/master/Dockerfile
# - https://kerkour.com/rust-small-docker-image#/from-buster-slim

FROM rust:latest as build
WORKDIR /usr/sample-graph-api
COPY . .
RUN cargo build --release

FROM debian:buster-slim as release
RUN apt-get update && apt-get install -y ca-certificates tzdata && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/sample-graph-api
COPY --from=build /usr/sample-graph-api/target/release/sample-graph-api sample-graph-api
CMD ["./sample-graph-api"]