FROM rustlang/rust:nightly as base
WORKDIR /src
COPY protos protos
WORKDIR /src/server
COPY dispatcher_service/dummy.rs .
COPY dispatcher_service/Cargo.toml .
RUN rustup component add rustfmt

FROM base as test
COPY dispatcher_service .
CMD [ "cargo", "test" ]

FROM base as build
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY dispatcher_service .
RUN cargo build --release

FROM rustlang/rust:nightly as runtime
COPY --from=build /src/server/target/release/dispatcher_service /usr/local/bin
EXPOSE 50051
ENTRYPOINT ["/usr/local/bin/dispatcher_service"]
