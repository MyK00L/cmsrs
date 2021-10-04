# cmsrs

A contest management system for local competitive programming contests like [ioi](https://ioinformatics.org/) or [icpc](https://icpc.global/).

Inspired by other similar projects like [cms](https://cms-dev.github.io/) and [DOMjudge](https://www.domjudge.org/).

The goals of this project are to be: easy to set up, fast, configurable; in that order.

For ease of use docker and docker-compose configuration files are provided.

## Dependencies:

### Docker dependencies:
With the use of docker, and end user, to set up a working environment, will only need:
- `docker`
- `docker-compose`

### Build dependencies:
Dependencies used to build the binaries outside of the ones managed by Cargo:
- `protoc` if prost-build does not have a precompiled protoc for your platform ([more info](https://lib.rs/crates/prost-build))

### Runtime dependencies:
Dependencies that will be used at runtime, not that if you are using docker, you should not have to worry about these.
- `mongodb` for `contest service` and `submission service`

## Project structure:

##### This project is subdivided in the following binaries:
- `submission service`: rpc service that manages submissions
- `evaluation service`: rpc service that manages evaluation files and configuration
- `contest service`: rpc service that manages participant communication, files and configuration
- `dispatcher service`: rpc service that dispatches submission evaluation to workers
- `worker service`: rpc service that evaluates submissions
- `admin webserver`: webserver for admin interaction
- `participant webserver`: webserver for participants
- `scoreboard webserver`: webserver that builds and shows a scoreboard

##### And the following libraries:
- `protos`: contains everything related to service communication
- `utils`: contains commonly needed utilities

