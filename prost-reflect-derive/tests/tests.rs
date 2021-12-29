#[test]
fn tests() {
    let tests = trybuild::TestCases::new();

    tests.pass("tests/basic.rs");
    tests.pass("tests/ignore_enum.rs");
    tests.pass("tests/package_name.rs");
    tests.pass("tests/multiple_attr.rs");
    tests.compile_fail("tests/attr_unknown_field.rs");
    tests.compile_fail("tests/missing_attr.rs");
    tests.compile_fail("tests/missing_name.rs");
}
