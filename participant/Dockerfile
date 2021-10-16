FROM rustlang/rust:nightly as base
WORKDIR /src
COPY protos protos
COPY utils utils
WORKDIR /src/server
COPY participant/dummy.rs .
COPY participant/Cargo.toml .
RUN rustup component add rustfmt

FROM base as build
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY participant .
RUN cargo build
# Generate web server secrets.
RUN ./rem_secrets.sh && ./gen_secrets.sh
# Expose web server port.
ENV ROCKET_PORT=8000
EXPOSE 8000
RUN cargo run --features mock
