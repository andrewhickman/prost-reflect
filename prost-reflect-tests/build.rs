use std::{env, io, path::PathBuf};

fn main() -> io::Result<()> {
    let file_descriptor_set_path =
        PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR environment variable not set"))
            .join("file_descriptor_set.bin");

    prost_build::Config::new()
        .type_attribute(".test", "#[derive(::proptest_derive::Arbitrary)]")
        .type_attribute(".test", "#[derive(::prost_reflect::ReflectMessage)]")
        .type_attribute(
            ".test.Scalars",
            "#[prost_reflect(file_descriptor = \"TEST_FILE_DESCRIPTOR\", message_name = \"test.Scalars\")]",
        )
        .type_attribute(
            ".test.ScalarArrays",
            "#[prost_reflect(file_descriptor = \"TEST_FILE_DESCRIPTOR\", message_name = \"test.ScalarArrays\")]",
        )
        .type_attribute(
            ".test.ComplexType",
            "#[prost_reflect(file_descriptor = \"TEST_FILE_DESCRIPTOR\", message_name = \"test.ComplexType\")]",
        )
        .type_attribute(
            ".test.WellKnownTypes",
            "#[prost_reflect(file_descriptor = \"TEST_FILE_DESCRIPTOR\", message_name = \"test.WellKnownTypes\")]",
        )
        .type_attribute(
            ".test.MessageWithOneof",
            "#[prost_reflect(file_descriptor = \"TEST_FILE_DESCRIPTOR\", message_name = \"test.MessageWithOneof\")]",
        )
        .type_attribute(
            ".test.Point",
            "#[prost_reflect(file_descriptor = \"TEST_FILE_DESCRIPTOR\", message_name = \"test.Point\")]",
        )
        .field_attribute(
            ".test.WellKnownTypes.timestamp",
            "#[proptest(strategy = \"::proptest::option::of(crate::arbitrary::timestamp())\")]",
        )
        .field_attribute(
            ".test.WellKnownTypes.duration",
            "#[proptest(strategy = \"::proptest::option::of(crate::arbitrary::duration())\")]",
        )
        .field_attribute(
            ".test.WellKnownTypes.struct",
            "#[proptest(strategy = \"::proptest::option::of(crate::arbitrary::struct_())\")]",
        )
        .field_attribute(
            ".test.WellKnownTypes.list",
            "#[proptest(strategy = \"::proptest::option::of(crate::arbitrary::list())\")]",
        )
        .field_attribute(
            ".test.WellKnownTypes.mask",
            "#[proptest(strategy = \"::proptest::option::of(crate::arbitrary::mask())\")]",
        )
        .field_attribute(
            ".test.WellKnownTypes.empty",
            "#[proptest(strategy = \"::proptest::option::of(::proptest::strategy::Just(()))\")]",
        )
        .field_attribute(".test.WellKnownTypes.null", "#[proptest(value= \"0\")]")
        .file_descriptor_set_path(file_descriptor_set_path)
        .compile_protos(
            &[
                "src/test.proto",
                "src/test2.proto",
                "src/desc.proto",
                "src/desc_no_package.proto",
            ],
            &["src/"],
        )?;
    Ok(())
}
