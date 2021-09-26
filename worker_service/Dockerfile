FROM rustlang/rust:nightly as base
WORKDIR /src 
COPY protos protos
# install the compilers needed 
# NB: rustc is already in the base image
RUN apt-get update && \
    apt-get install gcc
# finished installation of compilers
# package required by tabox 
RUN apt-get install -y libseccomp-dev 
WORKDIR /src/server
COPY worker_service/dummy.rs .
COPY worker_service/Cargo.toml .
RUN rustup component add rustfmt

FROM base as test
COPY worker_service .
CMD [ "cargo", "test" ]

FROM base as build
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY worker_service .
RUN cargo build --release

FROM rustlang/rust:nightly as runtime
COPY --from=build /src/server/target/release/worker_service /usr/local/bin
EXPOSE 50051
ENTRYPOINT ["/usr/local/bin/worker_service"]