FROM rustlang/rust:nightly as base
WORKDIR /src
COPY protos protos
WORKDIR /src/server
COPY rpctest-server/dummy.rs .
COPY rpctest-server/Cargo.toml .
RUN rustup component add rustfmt

FROM base as test
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY rpctest-server .
CMD [ "cargo", "test" ]

FROM base as build
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY rpctest-server .
RUN cargo build --release

FROM rustlang/rust:nightly as runtime
COPY --from=build /src/server/target/release/rpctest-server /usr/local/bin
EXPOSE 50051
ENTRYPOINT ["/usr/local/bin/rpctest-server"]
