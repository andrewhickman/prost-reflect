use std::io;

fn main() -> io::Result<()> {
    let mut config = prost_build::Config::new();
    config
        .type_attribute(".test.Scalars", "#[cfg_attr(test, derive(::proptest_derive::Arbitrary))]")
        .type_attribute(".test.ScalarArrays", "#[cfg_attr(test, derive(::proptest_derive::Arbitrary))]")
        .type_attribute(".test.ComplexType", "#[cfg_attr(test, derive(::proptest_derive::Arbitrary))]")
        .type_attribute(".test.WellKnownTypes", "#[cfg_attr(test, derive(::proptest_derive::Arbitrary))]")
        .field_attribute(
            ".test.WellKnownTypes.timestamp",
            "#[cfg_attr(test, proptest(strategy = \"::proptest::option::of(crate::arbitrary::timestamp())\"))]",
        )
        .field_attribute(
            ".test.WellKnownTypes.duration",
            "#[cfg_attr(test, proptest(strategy = \"::proptest::option::of(crate::arbitrary::duration())\"))]",
        )
        .field_attribute(
            ".test.WellKnownTypes.struct",
            "#[cfg_attr(test, proptest(strategy = \"::proptest::option::of(crate::arbitrary::struct_())\"))]",
        )
        .field_attribute(
            ".test.WellKnownTypes.list",
            "#[cfg_attr(test, proptest(strategy = \"::proptest::option::of(crate::arbitrary::list())\"))]",
        )
        .field_attribute(
            ".test.WellKnownTypes.mask",
            "#[cfg_attr(test, proptest(strategy = \"::proptest::option::of(crate::arbitrary::mask())\"))]",
        )
        .field_attribute(
            ".test.WellKnownTypes.empty",
            "#[cfg_attr(test, proptest(strategy = \"::proptest::option::of(::proptest::strategy::Just(()))\"))]",
        )
        .field_attribute(".test.WellKnownTypes.null", "#[cfg_attr(test, proptest(value= \"0\"))]");

    prost_reflect_build::Builder::new()
        .file_descriptor_expr("crate::TEST_DESCRIPTOR_POOL")
        .compile_protos_with_config(
            config,
            &[
                "src/test.proto",
                "src/test2.proto",
                "src/desc.proto",
                "src/desc2.proto",
                "src/desc_no_package.proto",
                "src/imports.proto",
                "src/ext.proto",
            ],
            &["src/"],
        )?;
    Ok(())
}
