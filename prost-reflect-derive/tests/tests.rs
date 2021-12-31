#[test]
fn tests() {
    let tests = trybuild::TestCases::new();

    tests.pass("tests/basic.rs");
    tests.pass("tests/ignore_enum.rs");
    tests.pass("tests/multiple_attr.rs");

    // Tarpaulin runs using the nightly compiler, which can result in different diagnostics
    if cfg!(not(tarpaulin)) {
        tests.compile_fail("tests/attr_unknown_field.rs");
        tests.compile_fail("tests/missing_attr.rs");
        tests.compile_fail("tests/missing_name.rs");
    }
}
