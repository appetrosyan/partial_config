#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
    t.pass("tests/ui/generic_lifetimes.rs");
    t.pass("tests/ui/env_option.rs");
}
