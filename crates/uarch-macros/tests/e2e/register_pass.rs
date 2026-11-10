#[test]
fn layout_contract() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/fail/register_pass_*.rs");
    cases.pass("tests/ui/pass/register_pass.rs");
}
