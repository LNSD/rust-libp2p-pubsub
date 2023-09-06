rust-libp2p-pubsub
---------
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)
[![ci](https://github.com/LNSD/rust-libp2p-pubsub/actions/workflows/ci.yml/badge.svg)](https://github.com/LNSD/rust-libp2p-pubsub/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/LNSD/rust-libp2p-pubsub/branch/main/graph/badge.svg?token=9UPTAJSD2U)](https://codecov.io/gh/LNSD/rust-libp2p-pubsub)

This is an alternative implementation of [rust-libp2p](https://github.com/libp2p/rust-libp2p)'s pubsub protocols.

> **Warning**
> This is a work in progress and is not ready for production use.

## Build requirements

To build this project you need the following:

- Rust toolchain (1.66+)
- [Buf CLI](https://docs.buf.build/installation)
- [Protobuf Compiler (protoc)](https://grpc.io/docs/protoc-installation/)
- [Protoc prost plugins (protoc-gen-prost)](https://github.com/neoeinstein/protoc-gen-prost):
    ```
    cargo install protoc-gen-prost
    cargo install protoc-gen-prost-crate
    ```
- Cargo nextest (Optional):
    ```
    cargo install cargo-nextest
    ```

## Protocol frame format

The Buf CLI is used to manage the protobuf files and generate the rust code. The code generation
is performed by the `build.rs` script and is run automatically when building the project.

> **Note**
> If the `cargo build` command fails, check first you have all the [build requirements](#build-requirements) installed.

## Supported Rust Versions

This repository is built against the latest stable release. The minimum supported
version is **1.66**. The current version is not guaranteed to build on Rust versions
earlier than the minimum supported version.

This project follows the same compiler support policies as the Tokio ecosystem.
The current stable Rust compiler and the three most recent minor versions before
it will always be supported. For example, if the current stable compiler version
is 1.69, the minimum supported version will not be increased past 1.66, three minor
versions prior. Increasing the minimum supported compiler version is not considered
a semantic versioning breaking change as long as doing so complies with this policy.

## License

<sup>
Licensed under either of <a href="LICENSE">Apache License, Version 2.0</a>.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be licensed as above, without any additional terms or conditions.
</sub>
