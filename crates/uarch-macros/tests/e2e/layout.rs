#[test]
fn conditional_nested_layouts() {
    trybuild::TestCases::new().pass("tests/ui/pass/hygiene.rs");
    trybuild::TestCases::new().pass("tests/ui/pass/layout.rs");
}
