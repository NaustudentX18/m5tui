use m5tui_core::*;
fn main() {
    // For the cockpit, find ANY opaque pixel in the agents list region
    // (rows 2-6, cols 1-18) to confirm the agents are rendered.
    let s = AppState::default();
    let theme = m5tui_core::default_theme();
    let f = render(&s, &theme);
    let buf = render_to_rgba(&f);
    let mut count = 0;
    let mut first = (0, 0);
    for row in 2..7 {
        for col in 1..19 {
            let cell_row = row;
            let cell_col = col;
            for gy in 0..8 {
                for gx in 0..6 {
                    let x = cell_col * 6 + gx;
                    let y = cell_row * 8 + gy;
                    let alpha = buf[(y * 240 + x) * 4 + 3];
                    if alpha == 0xFF {
                        if count == 0 {
                            first = (x, y);
                        }
                        count += 1;
                    }
                }
            }
        }
    }
    println!("opaque pixels in agents region: {count}, first at {first:?}");
    // Also check the prompt region (row 15, cols 0-15)
    let mut prompt_count = 0;
    let mut prompt_first = (0, 0);
    for x in 0..(15 * 6) {
        let y = 15 * 8;
        let alpha = buf[(y * 240 + x) * 4 + 3];
        if alpha == 0xFF {
            if prompt_count == 0 {
                prompt_first = (x, y);
            }
            prompt_count += 1;
        }
    }
    println!("opaque pixels in prompt region: {prompt_count}, first at {prompt_first:?}");
    // Also check cell (2, 0) which should be '>' for selected agent
    let c = f.cells[2][0];
    println!(
        "cell (2, 0): glyph=0x{:02x} ('{}') fg=0x{:04X}",
        c.glyph, c.glyph as char, c.fg
    );
    // And cell (2, 1) which should be 'a' of "aiserver-1/omp"
    let c = f.cells[2][1];
    println!(
        "cell (2, 1): glyph=0x{:02x} ('{}') fg=0x{:04X}",
        c.glyph, c.glyph as char, c.fg
    );
}
