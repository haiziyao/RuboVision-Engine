#[test]
fn attribute_macros_reject_invalid_usage() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
