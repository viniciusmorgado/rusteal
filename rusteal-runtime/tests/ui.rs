//! What the macros refuse at compile time, with the message the user reads.

#[test]
fn refused_at_compile_time() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
