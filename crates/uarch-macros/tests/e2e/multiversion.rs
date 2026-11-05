#[test]
fn public_contract() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass/multiversion.rs");
    cases.compile_fail("tests/ui/fail/multiversion_*.rs");
}
