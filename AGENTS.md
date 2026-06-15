# m5Tui — Agent Instructions

> This is the canonical entry point for any coding agent working on the
> m5Tui project. **Read this first.** It is the only file that explains
> the project to a subagent in a way that doesn't drift from the
> planning docs.

---

## 0. What m5Tui is

A single Rust binary that runs on the M5Stack Cardputer-Adv and acts as
a "Termius, but for agents" TUI. It SSHs into `aiserver-1` (Pi 5) over
Tailscale, embeds an OMP JSON-RPC client, captures voice via the
on-board mic, ships a theming engine with on-device customisation, and
is keyboard-first for a 240×135 ST7789V2 display.

**Project status:** PLANNING PHASE. No code has been written yet.
Approval from the SWAT is required before any code lands.

---

## 1. Read these, in order

1. `/home/pi/m5Tui/PLANNING.md` — the master SWAT brief. **Start here.**
2. `/home/pi/m5Tui/ARCHITECTURE.md` — the engineering spec, crate map, transport.
3. `/home/pi/m5Tui/FEATURES.md` — the complete feature inventory, milestone-tagged.
4. `/home/pi/m5Tui/THEMING.md` — the theming system, on-device editor, built-in themes.
5. `/home/pi/m5Tui/HARDWARE.md` — the Cardputer-Adv hardware reference, pin map, driver notes.
6. `/home/pi/m5Tui/ROADMAP.md` — the milestone breakdown, what ships in what order.

Then, **before doing any work**, read:

7. `/home/pi/m5-cardputer-adv/AGENTS.md` — the parent project's agent protocol.
8. `/home/pi/m5-cardputer-adv/docs/PRODUCT.md` — the adjacent product surface.
9. `/home/pi/m5-cardputer-adv/docs/ARCHITECTURE.md` — the data model we share.
10. `/home/pi/m5-cardputer-adv/docs/BRIDGE_PROTOCOL.md` — the JSON protocol on the server side.

You will *not* modify any file under `/home/pi/m5-cardputer-adv/`. You
are *consumers* of the bridge, not maintainers of it.

---

## 2. Hard rules

These are the rules that override any default behaviour:

1. **No code in `/home/pi/m5Tui/` until the SWAT approves PLANNING.md.**
   You may create or update docs. You may not create `src/`, `crates/`,
   `Cargo.toml`, or any executable code.
2. **No `git init` until the SWAT approves.** The folder exists on disk
   but is not a git repo. The SWAT picks the repo location and the
   license.
3. **No GitHub operations until the SWAT approves.** No `gh repo create`,
   no pushes, no issue creation.
4. **No flashing the Cardputer.** All driver code is reviewed against
   the spec in HARDWARE.md before it ever touches silicon.
5. **Do not modify the parent project.** `/home/pi/m5-cardputer-adv/`
   is read-only from this project's perspective.
6. **The reducer is pure.** `AppState -> Event -> AppState`. No I/O in
   the reducer. Side effects are queued as `Outgoing` actions and
   handled by the network/voice/persist loops.
7. **No `unsafe` outside `m5tui-voice` and `m5tui-status`.** Those two
   crates own the only `unsafe` in the codebase (ESP-I2S driver, AXP2101
   I2C). All cross-platform code is safe Rust.
8. **No global mutable state.** Everything lives in `AppState`.
9. **The theme is data, not code.** No `if theme == "cobalt"` anywhere
   in the codebase. If you need a per-theme code path, push the
   decision into the YAML.
10. **All persistent writes are atomic.** `write-temp + rename + fsync`.
    Schema migrations are explicit; old files are renamed to
    `<file>.v<n>.bak`, never silently overwritten.

---

## 3. Cargo workspace skeleton (DO NOT create until approved)

When the SWAT approves, the initial commit is:

```
/home/pi/m5Tui/
  Cargo.toml                              # workspace = [m5tui, ...]
  rust-toolchain.toml                     # pin to a stable rust + esp toolchain
  .gitignore                              # target/, .cargo/, *.tmp
  crates/
    m5tui-core/                           # UI shell, reducer, keymap
    m5tui-ssh/                            # russh client
    m5tui-omp/                            # OMP RPC client
    m5tui-voice/                          # audio in/out
    m5tui-themes/                         # theme loader + effects
    m5tui-pal/                            # command palette + fuzzy + snippets
    m5tui-profile/                        # profile registry
    m5tui-persist/                        # SD layout, atomic writes
    m5tui-status/                         # Wi-Fi, battery, doctor
  src/main.rs                             # tiny bin
  themes/                                 # cobalt, amber, stealth, etc.
  tests/
    unit/
    e2e/
    sim/
  docs/                                   # the .md files we already have
```

The `Cargo.toml` at the root should be a workspace manifest with no
`[package]`. The `m5tui` binary is in a tiny `src/main.rs` that just
bootstraps `m5tui-core::run`.

---

## 4. Testing contract

Every PR must pass:

- `cargo test --workspace` — unit tests for every crate.
- `cargo test --workspace --features integration` — e2e tests that
  spawn a local `sshd` in a container and a fake OMP server.
- `cargo test --workspace --test sim` — sim tests that render to a
  240×135 in-memory framebuffer and golden-diff against PNGs in
  `tests/sim/golden/`.
- `cargo clippy --workspace -- -D warnings` — no warnings.
- `cargo fmt --all -- --check` — formatted.

CI is GitHub Actions on push and PR. The matrix runs `native` and
`xtensa-esp32s3-espidf` builds.

---

## 5. Definition of Done

A task is done when:

- The requested behaviour or document exists.
- Tests are written and green (unit, e2e, sim as appropriate).
- `cargo clippy` and `cargo fmt` are clean.
- Docs are updated if the user-facing behaviour changes.
- File formats are documented.
- Failure modes are explicit.
- No unrelated repo churn is included.
- The CHANGELOG.md is updated.
- The ROADMAP.md's current-state header is updated.

---

## 6. Style

- **Rust 2021 edition.** Use modern idioms.
- **`rustfmt` defaults** with one tweak: `imports_granularity = "Crate"`.
- **`clippy::pedantic` is opt-in per crate** — turn it on for new code
  in the cross-platform crates (`m5tui-core`, `m5tui-themes`,
  `m5tui-pal`, `m5tui-persist`, `m5tui-profile`). Leave it off for
  `m5tui-voice` and `m5tui-status` (driver code; pedantic is noisy).
- **No emojis in code or comments** unless explicitly the user-facing
  string. The Cockpit screen uses some, but they're a themable asset
  in `/sd/m5tui/themes/`, not in the Rust source.
- **No `unwrap()` in library crates.** `expect` with a message is OK if
  the condition is genuinely unrecoverable. Tests can `unwrap`.
- **No `panic!` in library crates.** Return a `Result` or an error
  variant; let `m5tui-core` decide how to render it.

---

## 7. What to do when you start work

1. **Confirm the SWAT has approved PLANNING.md.** If not, stop and
   ask. Do not start coding.
2. Read all the docs in §1, in order.
3. Pick a task from ROADMAP.md. State the task ID in commits/PRs.
4. Make the change. Add tests. Update docs.
5. Run the full test matrix locally.
6. Push to your feature branch. Open a PR.
7. Hand off the PR URL for review.

---

## 8. What to do if you're confused

- The PLANNING.md has 10 open questions in §13. If your task hinges on
  one of them, **stop and ask** rather than picking unilaterally.
- The ARCHITECTURE.md has explicit "this is rejected" lists in §11. If
  you're tempted to break one of those rules, the plan has a reason.
- The HARDWARE.md has gotcha callouts. If your code is going near a
  shared I2C bus or a strapping pin, re-read §1.

---

**End of AGENTS.md.**
