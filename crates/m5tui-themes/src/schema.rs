//! Theme schema. The `Theme` struct is the value type that every widget,
//! render path, and editor screen consumes. The parser in `parser.rs`
//! produces one of these from a YAML string; `validate.rs` enforces
//! invariants (name length, brightness range, animation level) on top
//! of it. There is no interior mutability, no global state, no `unsafe`.
//!
//! All color values are stored as `Rgb565` (a `u16` newtype) so widgets
//! can write them directly into the framebuffer without any conversion
//! in the hot path.

/// A 16-bit RGB565 color value. Stored as a `u16` so it fits in a
/// framebuffer cell without conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb565(pub u16);

impl Rgb565 {
    /// Build a `Rgb565` from 8-bit-per-channel RGB. Bits 15..11 hold red
    /// (top 5 bits), 10..5 hold green (top 6 bits), 4..0 hold blue.
    pub const fn from_rgb888(r: u8, g: u8, b: u8) -> Self {
        let r = (r as u16 >> 3) & 0x001F;
        let g = (g as u16 >> 2) & 0x003F;
        let b = (b as u16 >> 3) & 0x001F;
        Self((r << 11) | (g << 5) | b)
    }

    /// Return the raw 16-bit value.
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// 24-bit packed hex value `0xRRGGBB`. Useful for YAML serialization.
    pub const fn to_rgb24(self) -> u32 {
        let (r, g, b) = self.to_rgb888();
        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }

    /// 24-bit RGB888 view: `(r, g, b)`. Useful for sim backends that
    /// render the framebuffer as 8-bit-per-channel.
    pub const fn to_rgb888(self) -> (u8, u8, u8) {
        let r = ((self.0 >> 11) & 0x001F) as u8;
        let g = ((self.0 >> 5) & 0x003F) as u8;
        let b = (self.0 & 0x001F) as u8;
        (
            (r << 3) | (r >> 2),
            (g << 2) | (g >> 4),
            (b << 3) | (b >> 2),
        )
    }
}

/// The 10-swatch palette. Every widget that needs a color reads from one
/// of these named slots. The slots are documented in `THEMING.md` §3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemePalette {
    /// Default background.
    pub bg: Rgb565,
    /// Default foreground (text).
    pub fg: Rgb565,
    /// Primary accent — titles, cursor, the "alive" color.
    pub accent: Rgb565,
    /// Dimmed text — timestamps, hints at rest.
    pub dim: Rgb565,
    /// Warning amber — low battery, soft alerts.
    pub warn: Rgb565,
    /// Hot red — errors, failures.
    pub err: Rgb565,
    /// Success green — OK state, confirmations.
    pub ok: Rgb565,
    /// Focused row background.
    pub sel_bg: Rgb565,
    /// Focused row foreground.
    pub sel_fg: Rgb565,
    /// Prompt cursor color.
    pub prompt: Rgb565,
}

/// Glyph set choice. `Heavy` is the hero for `Coldwire`; `Ascii` is the
/// `Lacuna` fallback that survives any font atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphSet {
    /// Plain ASCII box drawing: `+`, `-`, `|`. Fallback for monospace.
    Ascii,
    /// Heavy box drawing: `━━━ ┃┃ ┏━┓`. The `Coldwire` default.
    Heavy,
    /// Double box drawing: `═══ ║║ ╔═╗`. The `Phosphor` and `Magline` choice.
    Double,
    /// Rounded box drawing: `─── │ │ ╭─╮`. The `Noctilux` choice.
    Rounded,
}

/// Scrollbar visual style. Stored as data so a single render path can
/// branch on the theme's choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarStyle {
    /// Full block `█` with a `▌` thumb.
    Block,
    /// Thin line `│` with a `◆` thumb.
    Thin,
    /// Arrow `▲▼` with a `■` thumb.
    Arrow,
}

/// Cursor style for the prompt line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    /// Filled block `█`.
    Block,
    /// Bottom underline `▁`.
    Underline,
    /// Left bar `▎`.
    Bar,
}

/// Layout density. `Compact` is 40×16; `Comfy` is sparser. Widgets read
/// the field and pick the right grid math.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Density {
    /// 40×16, no extra spacing. The Cardputer native grid.
    Compact,
    /// 36×14, slightly looser.
    Cozy,
    /// 30×12, generous padding. For low-vision / legibility.
    Comfy,
}

/// Box-drawing, scrollbar, and cursor glyph choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeGlyphs {
    pub box_: GlyphSet,
    pub scrollbar: ScrollbarStyle,
    pub cursor: CursorStyle,
}

/// Layout tunables. `show_clock` and `show_synthwave` gate HUD elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeLayout {
    pub density: Density,
    pub show_clock: bool,
    pub show_synthwave: bool,
}

/// Animation level + per-effect toggles. `level` is the master dial
/// (0 = off, 1 = subtle, 2 = full cyberpunk). The booleans let a theme
/// mute individual effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeAnimation {
    /// 0..=2. `validate` rejects anything above 2.
    pub level: u8,
    pub scanline: bool,
    pub phosphor_decay: bool,
    pub glitch_on_event: bool,
}

/// Sound is per-theme: `enabled` is the master toggle. The remaining
/// fields are advisory paths the renderer reads when `enabled` is true;
/// empty strings mean "use the default boot/click/arp".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeSound {
    pub enabled: bool,
    pub boot: String,
    pub click: String,
    pub arp: String,
}

impl ThemeSound {
    /// Default `enabled = true` with no asset paths (caller picks).
    pub fn default_on() -> Self {
        Self {
            enabled: true,
            boot: String::new(),
            click: String::new(),
            arp: String::new(),
        }
    }
}

/// The top-level theme. A value type — no interior mutability, no
/// references, owned `String`s for the variable-length fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    /// Theme name, e.g. `"coldwire"`. Must be 1..32 chars (validated).
    pub name: String,
    /// Author handle, freeform. Empty string when omitted.
    pub author: String,
    /// 10-swatch palette.
    pub palette: ThemePalette,
    /// Box / scrollbar / cursor glyph choices.
    pub glyphs: ThemeGlyphs,
    /// Density + HUD toggles.
    pub layout: ThemeLayout,
    /// Animation level + per-effect flags.
    pub animation: ThemeAnimation,
    /// Sound enabled + asset paths.
    pub sound: ThemeSound,
    /// 0..=100. Drives the backlight PWM. `validate` rejects > 100.
    pub brightness: u8,
}
