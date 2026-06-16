//! Sim keymap tests: verify the chord parser, fuzzy matcher, and
//! side-channel reducer (`step`). These tests use only the public
//! `m5tui_core::*` re-exports.

use m5tui_core::*;

// -- parse_chord -----------------------------------------------------------

#[test]
fn parse_chord_empty_then_help() {
    // Bare `;` with no pending prefix is the Palette action (the input
    // layer stashes the `;` and calls parse_chord on the next key).
    assert_eq!(parse_chord(None, ';'), Some(KeyAction::Palette));
}

#[test]
fn parse_chord_palette_then_help() {
    assert_eq!(parse_chord(Some(';'), '?'), Some(KeyAction::Help));
}

#[test]
fn parse_chord_palette_then_palette() {
    assert_eq!(parse_chord(Some(';'), '/'), Some(KeyAction::Palette));
}

#[test]
fn parse_chord_palette_then_escape() {
    // `;;` is the escape for a literal `;` — the `;` prefix is consumed
    // and the second `;` passes through as a normal char.
    assert_eq!(parse_chord(Some(';'), ';'), Some(KeyAction::Char(';')));
}

#[test]
fn parse_chord_palette_then_char() {
    // Any other char after the `;` prefix passes through as a normal
    // char; the prefix is consumed.
    assert_eq!(parse_chord(Some(';'), 'c'), Some(KeyAction::Char('c')));
}

// -- fuzzy_score -----------------------------------------------------------

#[test]
fn fuzzy_score_empty_query() {
    assert_eq!(palette::fuzzy_score("", "connect"), 0);
}

#[test]
fn fuzzy_score_exact() {
    let s = palette::fuzzy_score("connect", "connect");
    assert!(s > 0, "exact match should score > 0, got {s}");
}

#[test]
fn fuzzy_score_prefix() {
    let c = palette::fuzzy_score("c", "connect");
    let n = palette::fuzzy_score("n", "connect");
    assert!(
        c > n,
        "prefix 'c' should score higher than mid-word 'n': c={c}, n={n}"
    );
}

#[test]
fn fuzzy_score_subsequence() {
    let s = palette::fuzzy_score("cnn", "connect");
    assert!(s > 0, "subsequence 'cnn' should score > 0, got {s}");
}

#[test]
fn fuzzy_score_case_insensitive() {
    let s = palette::fuzzy_score("CONNECT", "connect");
    assert!(s > 0, "case-insensitive prefix should score > 0, got {s}");
}

#[test]
fn fuzzy_score_no_match() {
    assert_eq!(palette::fuzzy_score("xyz", "connect"), 0);
}

#[test]
fn fuzzy_score_gap_penalty() {
    // A leading run of unmatched characters should pull the score down
    // via the gap penalty.
    let with_gap = palette::fuzzy_score("c", "xxxxxconnect");
    let direct = palette::fuzzy_score("c", "connect");
    assert!(
        with_gap < direct,
        "with-gap should score lower than direct: with_gap={with_gap}, direct={direct}"
    );
}

// -- filter ----------------------------------------------------------------

#[test]
fn filter_returns_sorted() {
    let hits = palette::filter("c", &palette::BUILTINS);
    assert!(!hits.is_empty(), "filter('c') should be non-empty");
    for w in hits.windows(2) {
        assert!(
            w[0].1 >= w[1].1,
            "filter result not sorted desc: {:?}",
            hits
        );
    }
}

#[test]
fn filter_query_too_obscure() {
    // A query with no characters present in any builtin name returns
    // nothing. "xqz" contains no x/q/z that appear in the 12 names.
    let hits = palette::filter("xqz", &palette::BUILTINS);
    assert!(hits.is_empty(), "filter('xqz') should be empty");
}

#[test]
fn filter_query_voice() {
    let hits = palette::filter("voi", &palette::BUILTINS);
    assert!(!hits.is_empty());
    let (top_idx, _top_score) = hits[0];
    assert_eq!(
        palette::BUILTINS[top_idx].name,
        "voice",
        "expected 'voice' at the top of filter('voi')"
    );
}

#[test]
fn filter_query_h_returns_h_prefix_matches() {
    // filter("h") returns names that start with `h` (case-insensitive).
    // `help` (index 8) and `handoff` (index 6) both qualify. On score
    // ties the smaller index wins, so `handoff` ranks above `help` in
    // the result. Both are present.
    let hits = palette::filter("h", &palette::BUILTINS);
    assert!(
        hits.len() >= 2,
        "expected at least 2 'h' matches, found {}",
        hits.len()
    );
    let names: Vec<&str> = hits
        .iter()
        .map(|(i, _)| palette::BUILTINS[*i].name)
        .collect();
    assert!(
        names.contains(&"help"),
        "expected 'help' in hits, got {names:?}"
    );
    assert!(
        names.contains(&"handoff"),
        "expected 'handoff' in hits, got {names:?}"
    );
}

// -- step (side-channel reducer) ------------------------------------------

#[test]
fn step_submit_prompt_returns_outgoing() {
    let s = AppState {
        prompt: "hello".into(),
        focus: Focus::Prompt,
        ..AppState::default()
    };
    let (new_s, out) = step(s, Event::Key(KeyAction::Enter));
    assert_eq!(out, vec![Outgoing::SubmitPrompt("hello".into())]);
    // The prompt is cleared after submit.
    assert_eq!(new_s.prompt, "");
}

#[test]
fn step_esc_from_palette_returns_cockpit() {
    let s = AppState {
        mode: Mode::Palette,
        ..AppState::default()
    };
    let (new_s, _out) = step(s, Event::Key(KeyAction::Esc));
    assert_eq!(new_s.mode, Mode::Cockpit);
}

#[test]
fn step_help_then_esc() {
    let s = AppState {
        mode: Mode::Help,
        ..AppState::default()
    };
    let (new_s, _out) = step(s, Event::Key(KeyAction::Esc));
    assert_eq!(new_s.mode, Mode::Cockpit);
}

#[test]
fn step_tab_cycles_focus() {
    let s = AppState {
        focus: Focus::Prompt,
        ..AppState::default()
    };
    // Prompt -> Agents.
    let (s1, out1) = step(s.clone(), Event::Key(KeyAction::Tab));
    assert_eq!(s1.focus, Focus::Agents);
    assert_eq!(out1, vec![Outgoing::CycleFocus(Focus::Agents)]);
    // Agents -> Session.
    let (s2, out2) = step(s1.clone(), Event::Key(KeyAction::Tab));
    assert_eq!(s2.focus, Focus::Session);
    assert_eq!(out2, vec![Outgoing::CycleFocus(Focus::Session)]);
    // Session -> Prompt.
    let (s3, out3) = step(s2.clone(), Event::Key(KeyAction::Tab));
    assert_eq!(s3.focus, Focus::Prompt);
    assert_eq!(out3, vec![Outgoing::CycleFocus(Focus::Prompt)]);
}

#[test]
fn step_tick_increments_clock() {
    let s = AppState::default();
    assert_eq!(s.clock, 0);
    let (s1, _out) = step(s, Event::Tick);
    assert_eq!(s1.clock, 1);
    let (s2, _out) = step(s1, Event::Tick);
    assert_eq!(s2.clock, 2);
}

#[test]
fn step_tick_drops_expired_toast() {
    // Toast that expired last tick: until_tick=1, clock=1, so after
    // the next Tick the reducer checks `state.clock >= t.until_tick`
    // against the *new* clock, which is 2, against 1 -> drop.
    let s = AppState {
        clock: 1,
        toast: Some(Toast {
            text: "expired".into(),
            until_tick: 1,
        }),
        ..AppState::default()
    };
    let (s2, _out) = step(s, Event::Tick);
    assert!(s2.toast.is_none(), "expired toast should be dropped");
}

#[test]
fn step_quit_sets_should_quit_and_emits_outgoing() {
    let s = AppState::default();
    let (s2, out) = step(s, Event::Quit);
    assert!(s2.should_quit);
    assert_eq!(out, vec![Outgoing::Quit]);
}

#[test]
fn step_close_overlay_event_returns_cockpit() {
    let s = AppState {
        mode: Mode::Palette,
        ..AppState::default()
    };
    let (s2, _out) = step(s, Event::CloseOverlay);
    assert_eq!(s2.mode, Mode::Cockpit);
}
