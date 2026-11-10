#[test]
fn public_contract() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass/isolated.rs");
    cases.compile_fail("tests/ui/fail/isolated_*.rs");
}
