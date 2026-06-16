use m5tui_core::*;
use std::path::PathBuf;

const RGBA_LEN: usize = 240 * 135 * 4;

fn render_and_save(state: &AppState, theme: &m5tui_themes::Theme, name: &str) {
    let frame = render(state, theme);
    let buf = sim::render_to_rgba(&frame);
    assert_eq!(
        buf.len(),
        RGBA_LEN,
        "expected {RGBA_LEN} bytes for 240x135 RGBA"
    );

    let out_dir = PathBuf::from("/tmp/m5tui-sim");
    std::fs::create_dir_all(&out_dir).unwrap_or_else(|e| panic!("create sim output dir: {e}"));
    let ppm = to_ppm(&buf);
    let path = out_dir.join(format!("{name}.ppm"));
    std::fs::write(&path, ppm).unwrap_or_else(|e| panic!("write ppm: {e}"));
    println!("m5Tui v1.0.0 -- {name} -> {}", path.display());
}

/// Convert RGBA8888 buffer to a PPM6 image (240x135).
fn to_ppm(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(32 + rgba.len() / 4 * 3);
    out.extend_from_slice(
        b"P6
240 135
255
",
    );
    for chunk in rgba.chunks_exact(4) {
        out.push(chunk[0]); // R
        out.push(chunk[1]); // G
        out.push(chunk[2]); // B
    }
    out
}

fn main() {
    let theme = default_theme();
    let s1 = AppState::default();
    render_and_save(&s1, &theme, "01-cockpit");

    let s2 = step(s1, Event::Key(KeyAction::Palette)).0;
    render_and_save(&s2, &theme, "02-palette");

    let s3 = step(s2, Event::Key(KeyAction::Help)).0;
    render_and_save(&s3, &theme, "03-help");

    let s4 = AppState {
        mode: Mode::ThemeEditor,
        ..s3
    };
    render_and_save(&s4, &theme, "04-theme-editor");

    println!("Open /tmp/m5tui-sim/*.ppm with any image viewer.");
}
