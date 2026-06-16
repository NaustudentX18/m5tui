//! Invalid theme rejection tests.
//!
//! The hand-rolled parser in `m5tui-themes` must reject inputs that
//! violate the schema (brightness out of range, an empty name, a list
//! at the top level, an empty document, etc.) and `validate` must accept
//! every shipped builtin.

#[test]
fn parse_rejects_brightness_out_of_range() {
    let yaml = "name: x\nbrightness: 200\n";
    let r = m5tui_themes::parse(yaml);
    assert!(
        r.is_err(),
        "expected parse to reject brightness=200, got {r:?}"
    );
}

#[test]
fn parse_rejects_top_level_list() {
    let yaml = "not yaml at all\n  - this is a list\n  - which we don't support";
    let r = m5tui_themes::parse(yaml);
    assert!(
        r.is_err(),
        "expected parse to reject a list at the top level, got {r:?}"
    );
}

#[test]
fn parse_rejects_empty_input() {
    let r = m5tui_themes::parse("");
    assert!(
        r.is_err(),
        "expected parse to reject empty input, got {r:?}"
    );
}

#[test]
fn validate_accepts_every_builtin() {
    for name in [
        "coldwire",
        "phosphor",
        "lacuna",
        "magline",
        "noctilux",
        "ivoryroom",
    ] {
        let t =
            m5tui_themes::builtin(name).unwrap_or_else(|| panic!("builtin({name}) returned None"));
        let r = m5tui_themes::validate(&t);
        assert!(r.is_ok(), "validate({name}) should be Ok, got {r:?}");
    }
}
