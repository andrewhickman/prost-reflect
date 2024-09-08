// This build script is based on the script here: https://github.com/tokio-rs/prost/blob/master/protobuf/build.rs

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use std::process::Command;
use std::{env, io::Read};

use anyhow::{Context, Result};
use flate2::bufread::GzDecoder;
use tar::Archive;

const VERSION: &str = "3.14.0";

static TEST_PROTOS: &[&str] = &["test_messages_proto2.proto", "test_messages_proto3.proto"];

fn main() -> Result<()> {
    let out_dir =
        &PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR environment variable not set"));
    let protobuf_dir = &out_dir.join(format!("protobuf-{}", VERSION));

    if !protobuf_dir.exists() {
        let tempdir = tempfile::Builder::new()
            .prefix("protobuf")
            .tempdir_in(out_dir)
            .expect("failed to create temporary directory");

        let src_dir = &download_protobuf(tempdir.path())?;
        let prefix_dir = &src_dir.join("prefix");
        fs::create_dir(prefix_dir).expect("failed to create prefix directory");
        install_conformance_test_runner(src_dir, prefix_dir)?;
        install_protos(src_dir, prefix_dir)?;
        fs::rename(prefix_dir, protobuf_dir).context("failed to move protobuf dir")?;
    }

    let include_dir = &protobuf_dir.join("include");
    let conformance_include_dir = include_dir.join("conformance");
    prost_build::compile_protos(
        &[conformance_include_dir.join("conformance.proto")],
        &[conformance_include_dir],
    )
    .unwrap();

    let test_includes = &include_dir.join("google").join("protobuf");
    prost_build::Config::new()
        .btree_map(["."])
        .file_descriptor_set_path(out_dir.join("test_messages.bin"))
        .compile_protos(
            &[
                test_includes.join("test_messages_proto2.proto"),
                test_includes.join("test_messages_proto3.proto"),
            ],
            &[include_dir],
        )
        .unwrap();

    // Emit an environment variable with the path to the build so that it can be located in the
    // main crate.
    println!("cargo:rustc-env=PROTOBUF={}", protobuf_dir.display());
    Ok(())
}

fn download_tarball(url: &str, out_dir: &Path) -> Result<()> {
    let mut data = Vec::new();
    ureq::get(url)
        .call()?
        .into_reader()
        .read_to_end(&mut data)?;

    // Unpack the tarball.
    Archive::new(GzDecoder::new(Cursor::new(data)))
        .unpack(out_dir)
        .context("failed to unpack tarball")
}

/// Downloads and unpacks a Protobuf release tarball to the provided directory.
fn download_protobuf(out_dir: &Path) -> Result<PathBuf> {
    download_tarball(
        &format!(
            "https://github.com/google/protobuf/archive/v{}.tar.gz",
            VERSION
        ),
        out_dir,
    )?;
    let src_dir = out_dir.join(format!("protobuf-{}", VERSION));

    Ok(src_dir)
}

#[cfg(windows)]
fn install_conformance_test_runner(_: &Path, _: &Path) -> Result<()> {
    // The conformance test runner does not support Windows [1].
    // [1]: https://github.com/protocolbuffers/protobuf/tree/master/conformance#portability
    Ok(())
}

#[cfg(not(windows))]
fn install_conformance_test_runner(src_dir: &Path, prefix_dir: &Path) -> Result<()> {
    // Apply patches.
    let mut patch_src = env::current_dir().context("failed to get current working directory")?;
    patch_src.push("fix-conformance_test_runner-cmake-build.patch");

    let rc = Command::new("patch")
        .arg("-p1")
        .arg("-i")
        .arg(patch_src)
        .current_dir(src_dir)
        .status()
        .context("failed to apply patch")?;
    anyhow::ensure!(rc.success(), "protobuf patch failed");

    // Build and install protoc, the protobuf libraries, and the conformance test runner.
    let rc = Command::new("cmake")
        .arg("-GNinja")
        .arg("cmake/")
        .arg("-DCMAKE_BUILD_TYPE=DEBUG")
        .arg(format!("-DCMAKE_INSTALL_PREFIX={}", prefix_dir.display()))
        .arg("-Dprotobuf_BUILD_CONFORMANCE=ON")
        .arg("-Dprotobuf_BUILD_TESTS=OFF")
        .current_dir(src_dir)
        .status()
        .context("failed to execute CMake")?;
    assert!(rc.success(), "protobuf CMake failed");

    let num_jobs = env::var("NUM_JOBS").context("NUM_JOBS environment variable not set")?;

    let rc = Command::new("ninja")
        .arg("-j")
        .arg(&num_jobs)
        .arg("install")
        .current_dir(src_dir)
        .status()
        .context("failed to execute ninja protobuf")?;
    anyhow::ensure!(rc.success(), "failed to make protobuf");

    fs::rename(
        src_dir.join("conformance_test_runner"),
        prefix_dir.join("bin").join("conformance-test-runner"),
    )
    .context("failed to move conformance-test-runner")?;

    Ok(())
}

fn install_protos(src_dir: &Path, prefix_dir: &Path) -> Result<()> {
    let include_dir = prefix_dir.join("include");

    // Move test protos to the prefix directory.
    let test_include_dir = &include_dir.join("google").join("protobuf");
    fs::create_dir_all(test_include_dir).expect("failed to create test include directory");
    for proto in TEST_PROTOS {
        fs::rename(
            src_dir
                .join("src")
                .join("google")
                .join("protobuf")
                .join(proto),
            test_include_dir.join(proto),
        )
        .with_context(|| format!("failed to move {}", proto))?;
    }

    // Move conformance.proto to the install directory.
    let conformance_include_dir = include_dir.join("conformance");
    fs::create_dir_all(&conformance_include_dir)
        .expect("failed to create conformance include directory");
    fs::rename(
        src_dir.join("conformance").join("conformance.proto"),
        conformance_include_dir.join("conformance.proto"),
    )
    .expect("failed to move conformance.proto");

    Ok(())
}
