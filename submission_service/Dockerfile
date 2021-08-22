FROM rustlang/rust:nightly as base
WORKDIR /src
COPY protos protos
WORKDIR /src/server
COPY submission_service/dummy.rs .
COPY submission_service/Cargo.toml .
RUN rustup component add rustfmt

FROM base as test
COPY submission_service .
CMD [ "cargo", "test" ]

FROM base as build
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY submission_service .
RUN cargo build --release

FROM rustlang/rust:nightly as runtime
COPY --from=build /src/server/target/release/submission_service /usr/local/bin
EXPOSE 50051
ENTRYPOINT ["/usr/local/bin/submission_service"]
