use std::env;
use std::path::PathBuf;
use std::process::{exit, Command};

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

fn main() {
    // TODO: Check build requirements here and print a user friendly error message
    //       if they are not met.

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
        eprintln!("Protobuf code generation failed: {}", status);
        exit(status.code().unwrap_or(1))
    }

    // Re-run this build script if any of the proto files have changed.
    println!("cargo:rerun-if-changed=**/*.proto");
}
