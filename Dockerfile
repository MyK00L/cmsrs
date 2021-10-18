### build base ###
#FROM lukemathwalker/cargo-chef:latest-rust as chef
FROM rust:1.55.0-slim AS chef
USER root
RUN rustup default nightly-2021-10-05
RUN rustup component add rustfmt
RUN cargo install cargo-chef
RUN apt-get update && apt-get install gcc && apt-get install libseccomp-dev
WORKDIR /app

### prepares chef for workspace ###
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

### builds workspace ###
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --workspace

### CONTEST SERVICE ###
FROM debian:buster-slim AS contest_service
COPY --from=builder /app/target/release/contest_service /usr/local/bin/
EXPOSE 50051
ENTRYPOINT ["/usr/local/bin/contest_service"]

### EVALUATION SERVICE ###
FROM debian:buster-slim AS evaluation_service
COPY --from=builder /app/target/release/evaluation_service /usr/local/bin/
EXPOSE 50051
ENTRYPOINT ["/usr/local/bin/evaluation_service"]

### DISPATCHER SERVICE ###
FROM debian:buster-slim AS dispatcher_service
COPY --from=builder /app/target/release/dispatcher_service /usr/local/bin/
EXPOSE 50051
ENTRYPOINT ["/usr/local/bin/dispatcher_service"]

### SUBMISSION SERVICE ###
FROM debian:buster-slim AS submission_service
COPY --from=builder /app/target/release/submission_service /usr/local/bin/
EXPOSE 50051
ENTRYPOINT ["/usr/local/bin/submission_service"]

### WORKER SERVICE ###
FROM debian:buster-slim AS worker_service
COPY --from=builder /app/target/release/worker_service /usr/local/bin/
EXPOSE 50051
RUN groupadd -g 1000 user && useradd -m -g 1000 -u 1000 user
ENTRYPOINT ["/usr/local/bin/worker_service"]

### ADMIN ###
FROM debian:buster-slim AS admin
RUN apt-get update && apt-get install openssl -y
COPY --from=builder /app/target/release/admin /usr/local/bin/
COPY --from=builder /app/admin /app/admin
WORKDIR /app/admin
RUN sh ./gen_secrets.sh
ENV ROCKET_PORT=80
RUN ls -lah
ENTRYPOINT ["/usr/local/bin/admin"]

### PARTICIPANT ###
FROM debian:buster-slim AS participant
RUN apt-get update && apt-get install openssl -y
COPY --from=builder /app/target/release/participant /usr/local/bin/
COPY --from=builder /app/participant /app/participant
WORKDIR /app/participant
RUN sh ./gen_secrets.sh
ENV ROCKET_PORT=80
ENTRYPOINT ["/usr/local/bin/participant"]

