//! The six built-in themes. Each YAML is embedded at compile time via
//! `include_str!` so the binary carries the whole set with no runtime
//! I/O. Tests assert the names in the order below; widget code looks
//! them up by name via `crate::builtin`.

/// The six built-in theme names, paired with their YAML sources, in
/// the order they appear in the picker.
pub const BUILTINS: &[(&str, &str)] = &[
    ("coldwire", include_str!("../../../themes/coldwire.yaml")),
    ("phosphor", include_str!("../../../themes/phosphor.yaml")),
    ("lacuna", include_str!("../../../themes/lacuna.yaml")),
    ("magline", include_str!("../../../themes/magline.yaml")),
    ("noctilux", include_str!("../../../themes/noctilux.yaml")),
    ("ivoryroom", include_str!("../../../themes/ivoryroom.yaml")),
];

/// All built-in names, in picker order. Convenience for code that just
/// wants the list (e.g. the theme editor's "load built-in" picker).
pub const BUILTIN_NAMES: &[&str] = &[
    "coldwire",
    "phosphor",
    "lacuna",
    "magline",
    "noctilux",
    "ivoryroom",
];
