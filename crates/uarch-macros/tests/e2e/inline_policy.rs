#[test]
fn public_contract() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass/inline_policy.rs");
    cases.compile_fail("tests/ui/fail/inline_policy_*.rs");
}
