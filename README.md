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

- Rust 1.65+
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

The proto files are located in a separate repository: https://github.com/LNSD/waku-proto/tree/rust-waku

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
