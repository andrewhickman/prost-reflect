#[test]
fn tests() {
    let tests = trybuild::TestCases::new();

    tests.pass("tests/basic.rs");
    tests.compile_fail("tests/attr_unknown_field.rs");
    tests.compile_fail("tests/duplicate_attr.rs");
    tests.compile_fail("tests/enum.rs");
    tests.compile_fail("tests/missing_attr.rs");
}
