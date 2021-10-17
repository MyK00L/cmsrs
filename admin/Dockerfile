FROM rustlang/rust:nightly as base
WORKDIR /src
COPY protos protos
COPY utils utils
WORKDIR /src/server
COPY admin/dummy.rs .
COPY admin/Cargo.toml .
RUN rustup component add rustfmt

RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY admin .
# Generate web server secrets.
RUN ./rem_secrets.sh && ./gen_secrets.sh
RUN cargo build
ENV ROCKET_PORT=80
# Add "--features", "mock" to the entrypoint list to use mock versions of other services.
ENTRYPOINT ["/src/server/target/debug/admin"]