FROM rust as builder
WORKDIR /src
COPY proto proto
WORKDIR /src/client
COPY rpctest-client/dummy.rs .
COPY rpctest-client/Cargo.toml .
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN rustup component add rustfmt
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY rpctest-client .
RUN cargo build --release

FROM rust as runtime
COPY --from=builder /src/client/target/release/rpctest-client /usr/local/bin
EXPOSE 50051
ENTRYPOINT ["/usr/local/bin/rpctest-client"]