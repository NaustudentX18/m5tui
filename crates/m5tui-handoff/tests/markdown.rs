//! Comprehensive tests for the `render_markdown` CommonMark-subset
//! renderer. Each test exercises a single element so a regression
//! points at the failing clause directly; the final combined-input
//! test catches interactions between branches.

use m5tui_handoff::{render_markdown, Line, LineStyle};

#[test]
fn heading_produces_heading_line() {
    let lines = render_markdown("# Foo");
    assert_eq!(lines.len(), 1, "expected exactly one line");
    assert_eq!(lines[0].style, LineStyle::Heading);
    assert_eq!(lines[0].text, "Foo");
}

#[test]
fn bold_and_italic_in_same_line_render_with_markers() {
    let lines = render_markdown("**foo** and *bar*");
    assert_eq!(lines.len(), 1);
    let text = &lines[0].text;
    assert!(
        text.contains("*foo*"),
        "bold should render with `*` markers, got {text:?}"
    );
    assert!(
        text.contains("_bar_"),
        "italic should render with `_` markers, got {text:?}"
    );
}

#[test]
fn unordered_list_two_items() {
    let lines = render_markdown("- a\n- b");
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].style, LineStyle::Bullet);
    assert_eq!(lines[1].style, LineStyle::Bullet);
    assert!(lines[0].text.contains('a'));
    assert!(lines[1].text.contains('b'));
}

#[test]
fn ordered_list_two_items() {
    let lines = render_markdown("1. a\n2. b");
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].style, LineStyle::Numbered);
    assert_eq!(lines[1].style, LineStyle::Numbered);
    assert!(lines[0].text.starts_with("1."));
    assert!(lines[1].text.starts_with("2."));
    assert!(lines[0].text.contains('a'));
    assert!(lines[1].text.contains('b'));
}

#[test]
fn fenced_code_block_lines_all_code() {
    let src = "```\nfoo\nbar\n```\n";
    let lines = render_markdown(src);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].style, LineStyle::Code);
    assert_eq!(lines[0].text, "foo");
    assert_eq!(lines[1].style, LineStyle::Code);
    assert_eq!(lines[1].text, "bar");
}

#[test]
fn long_word_wraps_at_38_columns() {
    // 80 'a' characters — longer than the 40-col framebuffer. The
    // renderer hard-breaks at column 38 with a `-` suffix.
    let word = "a".repeat(80);
    let lines = render_markdown(&word);
    assert!(
        lines.len() >= 2,
        "expected the word to wrap to at least two lines, got {}",
        lines.len()
    );
    let first = &lines[0].text;
    assert!(
        first.ends_with('-'),
        "first wrap line should end with `-`, got {first:?}"
    );
    assert!(
        first.len() <= 40,
        "first wrap line should fit in 40 cols, got len={}",
        first.len()
    );
}

#[test]
fn block_quote_renders_as_quote() {
    let lines = render_markdown("> quoted");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].style, LineStyle::Quote);
    assert_eq!(lines[0].text, "quoted");
}

#[test]
fn link_renders_as_text_with_url_parens() {
    let lines = render_markdown("[text](https://example.com)");
    assert_eq!(lines.len(), 1);
    let text = &lines[0].text;
    assert!(text.contains("text"), "link text missing: {text:?}");
    assert!(
        text.contains("(https://example.com)"),
        "link URL parens missing: {text:?}"
    );
}

#[test]
fn blank_input_line_renders_as_blank_style() {
    let lines = render_markdown("above\n\nbelow");
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].style, LineStyle::Normal);
    assert_eq!(lines[1].style, LineStyle::Blank);
    assert_eq!(lines[2].style, LineStyle::Normal);
}

#[test]
fn combined_input_exercises_every_feature() {
    // The order matters: assert position-by-position to catch any
    // reordering in the renderer pipeline.
    let src = "\
# Title

intro line

- bullet one
- bullet two

1. first
2. second

> a quote

```bash
$ echo hi
```

---

[docs](https://example.com)
";
    let lines = render_markdown(src);

    let mut cursor = 0;
    let assert_next =
        |lines: &[Line], cursor: &mut usize, expected_style: LineStyle, label: &str| {
            let line = lines
                .get(*cursor)
                .unwrap_or_else(|| panic!("{label}: ran out of lines at {cursor}"));
            assert_eq!(
                line.style, expected_style,
                "{label}: style mismatch at line {cursor}: {:?} vs {:?}",
                line.style, expected_style
            );
            *cursor += 1;
        };

    assert_next(&lines, &mut cursor, LineStyle::Heading, "title");
    assert_eq!(lines[cursor - 1].text, "Title");
    assert_next(&lines, &mut cursor, LineStyle::Blank, "blank after heading");
    assert_next(&lines, &mut cursor, LineStyle::Normal, "intro line");
    assert_next(&lines, &mut cursor, LineStyle::Blank, "blank after intro");
    assert_next(&lines, &mut cursor, LineStyle::Bullet, "bullet 1");
    assert_next(&lines, &mut cursor, LineStyle::Bullet, "bullet 2");
    assert_next(&lines, &mut cursor, LineStyle::Blank, "blank after bullets");
    assert_next(&lines, &mut cursor, LineStyle::Numbered, "numbered 1");
    assert_next(&lines, &mut cursor, LineStyle::Numbered, "numbered 2");
    assert_next(
        &lines,
        &mut cursor,
        LineStyle::Blank,
        "blank after numbered",
    );
    assert_next(&lines, &mut cursor, LineStyle::Quote, "quote line");
    assert_next(&lines, &mut cursor, LineStyle::Blank, "blank after quote");
    assert_next(&lines, &mut cursor, LineStyle::Code, "code block line");
    assert_next(&lines, &mut cursor, LineStyle::Blank, "blank after code");
    assert_next(&lines, &mut cursor, LineStyle::Rule, "horizontal rule");
    assert_next(&lines, &mut cursor, LineStyle::Blank, "blank after rule");
    assert_next(&lines, &mut cursor, LineStyle::Normal, "link line");
    assert!(
        lines[cursor - 1].text.contains("docs")
            && lines[cursor - 1].text.contains("https://example.com"),
        "link content missing: {:?}",
        lines[cursor - 1].text
    );
}
