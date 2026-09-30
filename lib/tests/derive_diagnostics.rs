#[test]
fn skipped_derive_items_report_diagnostics() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/skip_deserializing_field.rs");
    tests.compile_fail("tests/ui/skip_serializing_variant.rs");
    tests.compile_fail("tests/ui/dotted_collection_key_collision.rs");
    tests.compile_fail("tests/ui/dotted_alias_collection_key_collision.rs");
    tests.compile_fail("tests/ui/dotted_custom_multi_arg_collision.rs");
    tests.compile_fail("tests/ui/dotted_base_key_collision.rs");
    tests.compile_fail("tests/ui/dotted_serialize_only_collision.rs");
    tests.compile_fail("tests/ui/dotted_collection_self_alias_collision.rs");
    tests.compile_fail("tests/ui/dotted_shadowed_string_collision.rs");
    tests.compile_fail("tests/ui/dotted_shadowed_option_collision.rs");
    tests.compile_fail("tests/ui/dotted_reversed_multi_arg_collision.rs");
}
