#[test]
fn message_type_info_rejects_only_still_unsupported_shapes() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/message_type_info/enum.rs");
    cases.pass("tests/ui/message_type_info/option_field.rs");
    cases.compile_fail("tests/ui/message_type_info/const_generic.rs");
    cases.compile_fail("tests/ui/message_type_info/generic_enum.rs");
    cases.compile_fail("tests/ui/message_type_info/generic_tuple_struct.rs");
    cases.compile_fail("tests/ui/message_type_info/lifetime_generic.rs");
    cases.compile_fail("tests/ui/message_type_info/tuple_struct.rs");
}
