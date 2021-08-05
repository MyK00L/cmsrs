FROM rustlang/rust:nightly as base
WORKDIR /src
COPY protos protos
WORKDIR /src/client
COPY rpctest-client/dummy.rs .
COPY rpctest-client/Cargo.toml .
RUN rustup component add rustfmt

FROM base as test
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY rpctest-client .
CMD [ "cargo", "test" ]

FROM base as build
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY rpctest-client .
RUN cargo build --release

FROM rustlang/rust:nightly as runtime
COPY --from=build /src/client/target/release/rpctest-client /usr/local/bin
EXPOSE 50051
ENTRYPOINT ["/usr/local/bin/rpctest-client"]
