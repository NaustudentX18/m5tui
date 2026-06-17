//! Unit tests for `app.rs`. Lives in a sibling file so the source file
//! stays under the 350-line limit. Wired in via `#[path = "app_tests.rs"]
//! #[cfg(test)] mod tests;` in `app.rs`.

use super::*;
use crate::event::Focus;

#[test]
fn default_has_title() {
    assert_eq!(AppState::default().title, "m5Tui v0.1.0");
}

#[test]
fn default_has_zero_frame_count_and_no_quit() {
    let s = AppState::default();
    assert_eq!(s.frame_count, 0);
    assert!(!s.should_quit);
}

#[test]
fn default_has_five_mock_agents() {
    let s = AppState::default();
    assert_eq!(s.agents.len(), 5);
    assert_eq!(s.agents[0].name, "aiserver-1/omp");
    assert_eq!(s.agents[0].status, "ok");
    assert_eq!(s.agents[3].status, "warn");
    assert_eq!(s.agents[4].status, "down");
}

#[test]
fn default_mock_session_fields() {
    let s = AppState::default();
    assert_eq!(s.session.model, "MiniMax-M3");
    assert_eq!(s.session.task, "step 3/9 -- refactor");
    assert_eq!(s.session.in_flight, 2);
    assert_eq!(s.session.total, 9);
    assert!((s.session.rate_tps - 31.4).abs() < f32::EPSILON);
    assert_eq!(s.session.ctx_used, 12_300);
    assert_eq!(s.session.ctx_max, 512_000);
}

#[test]
fn reduce_tick_increments_frame_count() {
    let s = AppState::default();
    let s2 = reduce(s, Event::Tick);
    assert_eq!(s2.frame_count, 1);
}

#[test]
fn reduce_tick_increments_clock() {
    let s = AppState::default();
    let s2 = reduce(s, Event::Tick);
    assert_eq!(s2.clock, 1);
}

#[test]
fn reduce_quit_sets_should_quit() {
    let s = AppState::default();
    let s2 = reduce(s, Event::Quit);
    assert!(s2.should_quit);
}

#[test]
fn reduce_preserves_other_fields() {
    let s = AppState {
        title: "hello".into(),
        ..AppState::default()
    };
    let s2 = reduce(s.clone(), Event::Tick);
    assert_eq!(s2.title, "hello");
    assert_eq!(s2.frame_count, 1);
    assert!(!s2.should_quit);
}

#[test]
fn step_palette_key_opens_palette() {
    let s = AppState::default();
    let (s2, out) = step(s, Event::Key(KeyAction::Palette));
    assert_eq!(s2.mode, Mode::Palette);
    assert!(out.contains(&Outgoing::OpenPalette));
}

#[test]
fn step_help_key_opens_help() {
    let s = AppState::default();
    let (s2, out) = step(s, Event::Key(KeyAction::Help));
    assert_eq!(s2.mode, Mode::Help);
    assert!(out.contains(&Outgoing::OpenHelp));
}

#[test]
fn step_esc_from_help_returns_to_cockpit() {
    let s = AppState {
        mode: Mode::Help,
        ..AppState::default()
    };
    let (s2, _) = step(s, Event::Key(KeyAction::Esc));
    assert_eq!(s2.mode, Mode::Cockpit);
}

#[test]
fn step_esc_from_cockpit_focuses_prompt() {
    let s = AppState {
        focus: Focus::Agents,
        ..AppState::default()
    };
    let (s2, _) = step(s, Event::Key(KeyAction::Esc));
    assert_eq!(s2.mode, Mode::Cockpit);
    assert_eq!(s2.focus, Focus::Prompt);
}

#[test]
fn step_tab_cycles_focus() {
    let s = AppState::default();
    let (s2, out) = step(s, Event::Key(KeyAction::Tab));
    assert_eq!(s2.focus, Focus::Agents);
    assert!(out.contains(&Outgoing::CycleFocus(Focus::Agents)));
}

#[test]
fn step_tab_wraps_session_back_to_prompt() {
    let s = AppState {
        focus: Focus::Session,
        ..AppState::default()
    };
    let (s2, _) = step(s, Event::Key(KeyAction::Tab));
    assert_eq!(s2.focus, Focus::Prompt);
}

#[test]
fn step_char_in_prompt_appends() {
    let s = AppState::default();
    let (s2, _) = step(s, Event::Key(KeyAction::Char('h')));
    assert_eq!(s2.prompt, "h");
}

#[test]
fn step_char_in_agents_is_ignored() {
    let s = AppState {
        focus: Focus::Agents,
        ..AppState::default()
    };
    let (s2, _) = step(s, Event::Key(KeyAction::Char('h')));
    assert_eq!(s2.prompt, "");
}

#[test]
fn step_backspace_pops_last_char() {
    let s = AppState {
        prompt: "hi".into(),
        ..AppState::default()
    };
    let (s2, _) = step(s, Event::Key(KeyAction::Backspace));
    assert_eq!(s2.prompt, "h");
}

#[test]
fn step_enter_with_prompt_submits() {
    let s = AppState {
        prompt: "hello".into(),
        ..AppState::default()
    };
    let (s2, out) = step(s, Event::Key(KeyAction::Enter));
    assert!(out.contains(&Outgoing::SubmitPrompt("hello".into())));
    assert_eq!(s2.prompt, "");
}

#[test]
fn step_enter_with_empty_prompt_no_outgoing() {
    let s = AppState::default();
    let (_s2, out) = step(s, Event::Key(KeyAction::Enter));
    assert!(out.is_empty());
}

#[test]
fn step_up_in_agents_decrements() {
    let s = AppState {
        focus: Focus::Agents,
        selected_agent: 3,
        ..AppState::default()
    };
    let (s2, _) = step(s, Event::Key(KeyAction::Up));
    assert_eq!(s2.selected_agent, 2);
}

#[test]
fn step_up_in_agents_saturates_at_zero() {
    let s = AppState {
        focus: Focus::Agents,
        selected_agent: 0,
        ..AppState::default()
    };
    let (s2, _) = step(s, Event::Key(KeyAction::Up));
    assert_eq!(s2.selected_agent, 0);
}

#[test]
fn step_down_in_agents_increments() {
    let s = AppState {
        focus: Focus::Agents,
        selected_agent: 2,
        ..AppState::default()
    };
    let (s2, _) = step(s, Event::Key(KeyAction::Down));
    assert_eq!(s2.selected_agent, 3);
}

#[test]
fn step_down_in_agents_saturates_at_last() {
    let s = AppState {
        focus: Focus::Agents,
        selected_agent: 4,
        ..AppState::default()
    };
    let (s2, _) = step(s, Event::Key(KeyAction::Down));
    assert_eq!(s2.selected_agent, 4);
}

#[test]
fn step_enter_in_agents_emits_select() {
    let s = AppState {
        focus: Focus::Agents,
        selected_agent: 2,
        ..AppState::default()
    };
    let (_s2, out) = step(s, Event::Key(KeyAction::Enter));
    assert!(out.contains(&Outgoing::SelectAgent(2)));
}

#[test]
fn step_tick_increments_clock() {
    let s = AppState::default();
    let (s2, _) = step(s, Event::Tick);
    assert_eq!(s2.clock, 1);
}

#[test]
fn step_tick_drops_expired_toast() {
    let s = AppState {
        clock: 5,
        toast: Some(Toast {
            text: "x".into(),
            until_tick: 5,
        }),
        ..AppState::default()
    };
    let (s2, _) = step(s, Event::Tick);
    assert!(s2.toast.is_none());
}

#[test]
fn step_tick_keeps_unexpired_toast() {
    let s = AppState {
        clock: 3,
        toast: Some(Toast {
            text: "alive".into(),
            until_tick: 10,
        }),
        ..AppState::default()
    };
    let (s2, _) = step(s, Event::Tick);
    assert!(s2.toast.is_some());
    assert_eq!(s2.clock, 4);
}

#[test]
fn step_close_overlay_returns_to_cockpit() {
    let s = AppState {
        mode: Mode::Palette,
        ..AppState::default()
    };
    let (s2, _) = step(s, Event::CloseOverlay);
    assert_eq!(s2.mode, Mode::Cockpit);
}

#[test]
fn step_outgoing_close_overlay_returns_to_cockpit() {
    let s = AppState {
        mode: Mode::Help,
        ..AppState::default()
    };
    let (s2, _) = step(s, Event::Outgoing(Outgoing::CloseOverlay));
    assert_eq!(s2.mode, Mode::Cockpit);
}
#[test]
fn step_open_profile_picker_changes_mode() {
    let s = AppState::default();
    let (s2, outs) = step(s, Event::Key(KeyAction::OpenProfilePicker));
    assert_eq!(s2.mode, Mode::ProfilePicker);
    assert!(outs.contains(&Outgoing::OpenProfilePicker));
}

#[test]
fn step_open_book_changes_mode() {
    let s = AppState::default();
    let (s2, outs) = step(s, Event::Key(KeyAction::OpenBook));
    assert_eq!(s2.mode, Mode::Book);
    assert!(outs.contains(&Outgoing::OpenBook));
}

#[test]
fn step_open_voice_changes_mode() {
    let s = AppState::default();
    let (s2, outs) = step(s, Event::Key(KeyAction::OpenVoice));
    assert_eq!(s2.mode, Mode::Voice);
    assert!(outs.contains(&Outgoing::OpenVoice));
}

#[test]
fn step_open_first_boot_changes_mode() {
    let s = AppState::default();
    let (s2, outs) = step(s, Event::Key(KeyAction::OpenFirstBoot));
    assert_eq!(s2.mode, Mode::FirstBoot);
    assert!(outs.contains(&Outgoing::OpenFirstBoot));
}

#[test]
fn step_open_settings_changes_mode() {
    let s = AppState::default();
    let (s2, outs) = step(s, Event::Key(KeyAction::OpenSettings));
    assert_eq!(s2.mode, Mode::Settings);
    assert!(outs.contains(&Outgoing::OpenSettings));
}

#[test]
fn step_run_doctor_changes_mode() {
    let s = AppState::default();
    let (s2, outs) = step(s, Event::Key(KeyAction::RunDoctor));
    assert_eq!(s2.mode, Mode::Doctor);
    assert!(outs.contains(&Outgoing::RunDoctor));
}

#[test]
fn step_open_handoff_changes_mode() {
    let s = AppState::default();
    let (s2, outs) = step(s, Event::Key(KeyAction::OpenHandoff));
    assert_eq!(s2.mode, Mode::Handoff);
    assert!(outs.contains(&Outgoing::OpenHandoff));
}

#[test]
fn step_open_memory_changes_mode() {
    let s = AppState::default();
    let (s2, outs) = step(s, Event::Key(KeyAction::OpenMemory));
    assert_eq!(s2.mode, Mode::Memory);
    assert!(outs.contains(&Outgoing::OpenMemory));
}

#[test]
fn step_save_memo_emits_outgoing() {
    let s = AppState {
        prompt: "remember the milk".into(),
        ..AppState::default()
    };
    let (s2, outs) = step(s, Event::Key(KeyAction::SaveMemo));
    assert!(outs.contains(&Outgoing::SaveMemo("remember the milk".into())));
    assert_eq!(s2.prompt, "");
}

#[test]
fn step_save_memo_ignores_empty_prompt() {
    let s = AppState {
        prompt: "".into(),
        ..AppState::default()
    };
    let (s2, outs) = step(s, Event::Key(KeyAction::SaveMemo));
    assert!(outs.is_empty());
    assert_eq!(s2.prompt, "");
}

#[test]
fn step_overlay_open_ignored_when_not_in_cockpit() {
    // Once an overlay is open, opening another should be a no-op.
    let s = AppState {
        mode: Mode::Palette,
        ..AppState::default()
    };
    let (s2, outs) = step(s, Event::Key(KeyAction::OpenBook));
    assert_eq!(s2.mode, Mode::Palette);
    assert!(!outs.contains(&Outgoing::OpenBook));
}

#[test]
fn step_fork_draft_theme_creates_draft() {
    let s = AppState::default();
    let (s2, _) = step(s, Event::Key(KeyAction::ForkDraftTheme));
    assert!(s2.theme_draft.is_some());
    assert_eq!(s2.theme_draft.as_ref().unwrap().name, "coldwire");
}

#[test]
fn step_fork_draft_theme_idempotent() {
    let s = AppState::default();
    let (s2, _) = step(s, Event::Key(KeyAction::ForkDraftTheme));
    let draft1 = s2.theme_draft.clone();
    let (s3, _) = step(s2, Event::Key(KeyAction::ForkDraftTheme));
    assert_eq!(s3.theme_draft, draft1);
}

#[test]
fn step_commit_draft_theme_emits_save() {
    let s = AppState::default();
    let (s2, _) = step(s, Event::Key(KeyAction::ForkDraftTheme));
    let (s3, outs) = step(s2, Event::Key(KeyAction::CommitDraftTheme));
    assert!(s3.theme_draft.is_none());
    let has_save = outs.iter().any(|o| matches!(o, Outgoing::SaveTheme(_)));
    assert!(has_save, "expected Outgoing::SaveTheme in {outs:?}");
}

#[test]
fn step_discard_draft_theme_clears_without_save() {
    let s = AppState::default();
    let (s2, _) = step(s, Event::Key(KeyAction::ForkDraftTheme));
    let (s3, outs) = step(s2, Event::Key(KeyAction::DiscardDraftTheme));
    assert!(s3.theme_draft.is_none());
    let has_save = outs.iter().any(|o| matches!(o, Outgoing::SaveTheme(_)));
    assert!(!has_save);
}
