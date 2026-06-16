//! v1.0 integration smoke test.
//!
//! Exercises every crate in the workspace through their public APIs:
//! themes, render, profile, persist, ssh, omp, voice, book, market,
//! handoff. No real network or hardware is touched.

use m5tui_book::{BookRegistry, InMemoryRegistry as BookRegistryImpl, Spell, UndoStack};
use m5tui_handoff::{render_markdown, HandoffStore, InMemoryStore as HandoffStoreImpl};
use m5tui_market::{MarketClient, StubMarketClient};
use m5tui_omp::{OmpFrame, OmpSession, StubOmpSession};
use m5tui_persist::{load_theme, save_theme, MemoryDriver};
use m5tui_profile::{InMemoryRegistry as ProfileRegistryImpl, Profile, ProfileRegistry};
use m5tui_ssh::{SshClient, StubSshClient};
use m5tui_voice::Wav;

#[test]
fn v1_pipeline_smoke() {
    // Theme: load coldwire, render cockpit, persist round-trip.
    let theme = m5tui_themes::builtin("coldwire").unwrap_or_else(|| panic!("coldwire"));
    let state = m5tui_core::AppState::default();
    let frame = m5tui_core::render(&state, &theme);
    assert_eq!(frame.cells.len(), m5tui_core::layout::ROWS);

    let mut mem = MemoryDriver::new();
    save_theme(&mut mem, "/themes/coldwire.yaml", &theme).unwrap_or_else(|e| panic!("{e}"));
    let reloaded = load_theme(&mem, "/themes/coldwire.yaml").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(reloaded.name, "coldwire");

    // Profile + SSH.
    let mut profiles = ProfileRegistryImpl::new();
    profiles
        .add(Profile {
            id: "aiserver-1".to_string(),
            label: "aiserver".to_string(),
            host: "aiserver-1.tailnet.ts.net".to_string(),
            user: "pi".to_string(),
            key_path: "/key".to_string(),
            jump_profile_id: None,
            omp_profile: "default".to_string(),
        })
        .unwrap_or_else(|e| panic!("{e}"));
    profiles
        .set_default("aiserver-1")
        .unwrap_or_else(|e| panic!("{e}"));
    let profile = profiles
        .default()
        .unwrap_or_else(|| panic!("default"))
        .clone();

    let mut ssh = StubSshClient::new();
    ssh.connect(&profile).unwrap_or_else(|e| panic!("{e}"));
    let hostname = ssh.exec("hostname").unwrap_or_else(|e| panic!("{e}"));
    assert!(hostname.contains("aiserver"));

    // OMP session.
    let mut omp = StubOmpSession::simple();
    omp.start().unwrap_or_else(|e| panic!("{e}"));
    let ask = OmpFrame::Ask {
        id: "1".to_string(),
        text: "hello".to_string(),
    };
    omp.send(&ask).unwrap_or_else(|e| panic!("{e}"));
    let mut answers = 0usize;
    while let Ok(Some(frame)) = omp.recv() {
        if matches!(frame, OmpFrame::Answer { .. }) {
            answers += 1;
        }
    }
    assert_eq!(answers, 1);

    // Voice WAV round-trip.
    let samples: Vec<i16> = (0..64).map(|i| i as i16 * 100).collect();
    let wav = Wav::encode_mono_pcm16(&samples, 16000);
    let (decoded, rate) = Wav::decode_mono_pcm16(&wav).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(rate, 16000);
    assert_eq!(decoded, samples);

    // Book registry + expand + undo.
    let mut book = BookRegistryImpl::new();
    book.add(Spell {
        name: "ask".to_string(),
        key: "a".to_string(),
        prefix: ";".to_string(),
        help: "ask".to_string(),
        params: vec![],
        template: r##"omp ask \"{question}\""##.to_string(),
        undo_template: Some("echo undo {question}".to_string()),
    })
    .unwrap_or_else(|e| panic!("{e}"));
    let spell = book.get("ask").unwrap_or_else(|| panic!("ask"));
    let mut values = std::collections::HashMap::new();
    values.insert("question".to_string(), "rust?".to_string());
    let expanded = spell.expand(&values);
    assert!(expanded.contains("rust?"));
    let mut undo = UndoStack::new();
    if let Some(u) = spell.expand_undo(&values) {
        undo.push(u);
    }
    assert_eq!(undo.len(), 1);

    // Market preview.
    let market = StubMarketClient::with_canned();
    let catalog = market.fetch_catalog().unwrap_or_else(|e| panic!("{e}"));
    assert!(!catalog.entries.is_empty());

    // Handoff + markdown.
    let handoffs = HandoffStoreImpl::with_samples();
    let hits = handoffs.search("m5Tui");
    assert!(!hits.is_empty());
    let lines = render_markdown(
        "# Title
- item",
    );
    assert!(!lines.is_empty());
}
