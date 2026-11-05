#[test]
fn public_contract() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass/cache_line.rs");
    cases.compile_fail("tests/ui/fail/cache_line_*.rs");
}
