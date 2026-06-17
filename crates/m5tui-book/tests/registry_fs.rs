// Integration tests for `load_book_dir` and `FileRegistry`.
//
// These exercise the public filesystem surface against real temp
// directories. No `tempfile` dep is available in this crate, so we
// build unique temp dirs from `std::env::temp_dir()` and clean up
// after each test. Tests run in parallel, so each picks its own
// PID/timestamp/nanos suffix.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use m5tui_book::{load_book_dir, BookRegistry, FileRegistry, Spell};

const PLAN_YAML: &str = "\
name: plan
key: p
prefix: \";\"
help: Generate a plan via OMP
params:
  - name: goal
    prompt: Goal
    required: true
template: 'omp plan \"{goal}\"'
";

const ASK_YAML: &str = "\
name: ask
key: a
prefix: \";\"
help: Ask OMP a question
params:
  - name: question
    prompt: Question
    required: true
template: 'omp ask \"{question}\"'
";

const INVALID_YAML: &str = "\
name: 1
key: a
";

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_tmp(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id() as u128;
    let seq = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("m5tui-book-{label}-{pid}-{nanos}-{seq}"));
    fs::create_dir_all(&dir).expect("create tempdir");
    dir
}

fn write_file(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).expect("write file");
    path
}

fn names(spells: &[&Spell]) -> Vec<String> {
    let mut out: Vec<String> = spells.iter().map(|s| s.name.clone()).collect();
    out.sort();
    out
}

#[test]
fn load_book_dir_reads_two_yaml_files() {
    let dir = unique_tmp("load-two");
    write_file(&dir, "ask.yaml", ASK_YAML);
    write_file(&dir, "plan.yaml", PLAN_YAML);

    let registry = load_book_dir(&dir).expect("load_book_dir ok");
    let listed = registry.list();
    assert_eq!(listed.len(), 2, "expected 2 spells, got {}", listed.len());
    assert_eq!(names(&listed), vec!["ask", "plan"]);

    let ask = registry.get("ask").expect("ask present");
    assert_eq!(ask.key, "a");
    assert_eq!(ask.template, "omp ask \"{question}\"");

    let plan = registry.get("plan").expect("plan present");
    assert_eq!(plan.key, "p");
    assert_eq!(plan.template, "omp plan \"{goal}\"");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn load_book_dir_empty_dir_is_ok() {
    let dir = unique_tmp("empty");
    let registry = load_book_dir(&dir).expect("empty dir ok");
    assert_eq!(registry.list().len(), 0);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn load_book_dir_invalid_yaml_returns_parse_error_with_path() {
    let dir = unique_tmp("invalid");
    let path = write_file(&dir, "bad.yaml", INVALID_YAML);

    let err = load_book_dir(&dir).expect_err("invalid yaml should fail");
    let msg = err.to_string();
    assert!(
        msg.contains(&path.display().to_string()),
        "error must mention the file path; got: {msg}"
    );
    // The file has `name: 1` and `key: a` but no template — the
    // structural `Spell::validate` failure bubbles up under our
    // parse-error envelope so the file path is preserved.
    assert!(
        msg.contains("parse error"),
        "error should look like a parse error; got: {msg}"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn file_registry_new_loads_two_spells() {
    let dir = unique_tmp("fr-new");
    write_file(&dir, "ask.yaml", ASK_YAML);
    write_file(&dir, "plan.yaml", PLAN_YAML);

    let fr = FileRegistry::new(&dir).expect("FileRegistry::new ok");
    assert_eq!(fr.spell_count(), 2);
    let list = fr.list();
    assert_eq!(names(&list), vec!["ask", "plan"]);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn file_registry_refresh_if_stale_picks_up_edit() {
    let dir = unique_tmp("fr-stale");
    let path = write_file(&dir, "ask.yaml", ASK_YAML);
    write_file(&dir, "plan.yaml", PLAN_YAML);

    let mut fr = FileRegistry::new(&dir).expect("FileRegistry::new ok");
    assert_eq!(fr.spell_count(), 2);
    let before_template = fr
        .registry()
        .get("ask")
        .expect("ask present")
        .template
        .clone();

    // Rewrite the file with a different template body. Some
    // filesystems have second-granular mtimes; sleep past one
    // second so the new mtime is strictly greater. This keeps the
    // test dep-free (no `filetime` crate available).
    thread::sleep(Duration::from_millis(1100));
    let rewritten = ASK_YAML.replace(
        "template: 'omp ask \"{question}\"'\n",
        "template: 'omp ask \"{question} v2\"'\n",
    );
    assert_ne!(rewritten, ASK_YAML);
    fs::write(&path, &rewritten).expect("rewrite file");

    let changed = fr.refresh_if_stale().expect("refresh ok");
    assert!(changed, "refresh_if_stale should report a change");
    let after_template = fr
        .registry()
        .get("ask")
        .expect("ask still present")
        .template
        .clone();
    assert_ne!(
        after_template, before_template,
        "spell template must reflect the rewritten file"
    );
    assert_eq!(after_template, "omp ask \"{question} v2\"");
    assert_eq!(fr.spell_count(), 2);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn file_registry_impls_book_registry_trait() {
    let dir = unique_tmp("fr-trait");
    write_file(&dir, "ask.yaml", ASK_YAML);
    write_file(&dir, "plan.yaml", PLAN_YAML);

    let fr = FileRegistry::new(&dir).expect("FileRegistry::new ok");
    // Exercise the trait via `&dyn BookRegistry` dispatch to confirm
    // the impl is real, not stubbed.
    let registry: &dyn BookRegistry = &fr;
    let listed = registry.list();
    assert_eq!(listed.len(), 2);
    assert!(registry.get("ask").is_some());
    assert!(registry.get("plan").is_some());
    assert!(registry.get("nope").is_none());

    fs::remove_dir_all(&dir).ok();
}
