use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Return the path to root of the crate being built.
///
/// The `CARGO_MANIFEST_DIR` env variable contains the path to the  directory containing the
/// manifest for the package being built (the package containing the build script). Also note that
/// this is the value of the current working directory of the build script when it starts.
///
/// https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
fn root_dir() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
}

/// Check if all the build requirements are met.
///
/// This function checks if the following tools are installed:
/// - Buf CLI
/// - protoc
/// - protoc-gen-prost
/// - protoc-gen-prost-crate
fn check_build_requirements() -> Result<(), String> {
    let mut errors = vec![];

    // Check if buf is installed.
    let buf = Command::new("buf").arg("--version").status().unwrap();
    if !buf.success() {
        errors.push("Buf CLI not found. Please install buf: https://docs.buf.build/installation");
    }

    // Check if protoc is installed.
    let protoc = Command::new("protoc").arg("--version").status().unwrap();
    if !protoc.success() {
        errors.push(
            "protoc not found. Please install protoc: https://grpc.io/docs/protoc-installation/",
        );
    }

    // Check if protoc-gen-prost is installed.
    let protoc_gen_prost = Command::new("protoc-gen-prost")
        .arg("--version")
        .status()
        .unwrap();
    if !protoc_gen_prost.success() {
        errors.push("protoc-gen-prost not found. Please install protoc-gen-prost: cargo install protoc-gen-prost --locked");
    }

    // Check if protoc-gen-prost-crate is installed.
    let protoc_gen_prost_crate = Command::new("protoc-gen-prost-crate")
        .arg("--version")
        .status()
        .unwrap();
    if !protoc_gen_prost_crate.success() {
        errors.push("protoc-gen-prost-crate not found. Please install protoc-gen-prost-crate: cargo install protoc-gen-prost-crate --locked");
    }

    if !errors.is_empty() {
        return Err(format!(
            "Build requirements not met:\n - {}",
            errors.join("\n - ")
        ));
    }

    Ok(())
}

fn main() {
    // Run code generation only if 'proto-gen' feature is enabled.
    if env::var("CARGO_FEATURE_PROTO_GEN").is_err() {
        println!("Skipping code generation. 'proto-gen' feature is not enabled.");
        return;
    }

    // Check if all the build requirements are met.
    if let Err(err) = check_build_requirements() {
        panic!("{}", err);
    }

    let src_dir = root_dir().join("src");
    let src_gen_dir = root_dir().join("src/gen");
    let proto_dir = root_dir().join("proto");
    let buf_gen_yaml = root_dir().join("proto/buf.gen.yaml");

    // Remove the generated code directory if it exists.
    if src_gen_dir.exists() {
        std::fs::remove_dir_all(&src_gen_dir).unwrap();
    }

    // Run `buf generate` to generate the protobuf code. See Buf CLI's generate documentation for
    // more information: https://buf.build/docs/generate/overview
    let status = Command::new("buf")
        .arg("generate")
        .arg("--template")
        .arg(&buf_gen_yaml)
        .arg("--output")
        .arg(&src_dir)
        .current_dir(&proto_dir)
        .status()
        .unwrap();

    if !status.success() {
        panic!("Protobuf code generation failed: {}", status);
    }

    // Re-run this build script if any of the proto files have changed.
    println!("cargo:rerun-if-changed=**/*.proto");
}
