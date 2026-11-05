#[test]
fn public_contract() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass/pattern.rs");
    cases.compile_fail("tests/ui/fail/pattern_*.rs");
}
