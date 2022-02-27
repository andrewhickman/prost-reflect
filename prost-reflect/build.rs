#[cfg(feature = "reflect-well-known-types")]
fn main() -> std::io::Result<()> {
    use std::{env, path::PathBuf};

    let mut wkt_path =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR environment variable not set"));
    wkt_path.push("well_known_types.bin");

    let protos = &[
        "google/protobuf/any.proto",
        "google/protobuf/api.proto",
        "google/protobuf/descriptor.proto",
        "google/protobuf/empty.proto",
        "google/protobuf/duration.proto",
        "google/protobuf/field_mask.proto",
        "google/protobuf/source_context.proto",
        "google/protobuf/struct.proto",
        "google/protobuf/timestamp.proto",
        "google/protobuf/type.proto",
        "google/protobuf/wrappers.proto",
        "google/protobuf/compiler/plugin.proto",
    ];
    let includes: &[&str] = &[];
    prost_build::Config::new()
        .file_descriptor_set_path(&wkt_path)
        .compile_protos(protos, includes)
}

#[cfg(not(feature = "reflect-well-known-types"))]
fn main() {}
