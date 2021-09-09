FROM rustlang/rust:nightly as base
WORKDIR /src
COPY protos protos
COPY utils utils
WORKDIR /src/server
COPY contest_service/dummy.rs .
COPY contest_service/Cargo.toml .
RUN rustup component add rustfmt

FROM base as test
COPY contest_service .
CMD [ "cargo", "test" ]

FROM base as build
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY contest_service .
RUN cargo build --release

FROM rustlang/rust:nightly as runtime
COPY --from=build /src/server/target/release/contest_service /usr/local/bin
EXPOSE 50051
ENTRYPOINT ["/usr/local/bin/contest_service"]
