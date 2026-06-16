//! Color palette (RGB565) + builtin command descriptors and a tiny fuzzy
//! matcher for the M1 command palette.
//!
//! The color block is the single source of truth for RGB565 constants used
//! across the widgets. The matcher block below is intentionally simple: no
//! regex, no Levenshtein, no external crates. It scores a `(query, name)`
//! pair as a sum of bonuses (exact match, prefix, subsequence) minus a gap
//! penalty. The result is always `>= 0`. `filter` walks the builtin list,
//! scores each entry, and returns the positive-score matches sorted by
//! score descending.
// --- Color palette (RGB565). The widget layer is the only consumer; the
// --- input layer never references colors. Constants live in this module so
// --- `use m5tui_core::palette::*` resolves both the color block and the
// --- command block below.

/// RGB565 black. Default background of the cockpit/help screens.
pub const BG: u16 = 0x0000;
/// RGB565 white. Default foreground.
pub const FG: u16 = 0xFFFF;
/// RGB565 cyan (R=0, G=31, B=31). The primary accent — titles, selected
/// rows, numbers.
pub const CYAN: u16 = 0x07FF;
/// RGB565 magenta (R=31, G=0, B=31). Secondary accent for paired callouts.
pub const MAGENTA: u16 = 0xF81F;
/// RGB565 yellow (R=31, G=63, B=0). Warning / toast foreground.
pub const YELLOW: u16 = 0xFFE0;
/// RGB565 50% gray. Dimmed text — `down` status, hints at rest.
pub const DIM: u16 = 0x7BEF;
/// RGB565 dark blue. Selection background for the modal palette.
pub const SEL_BG: u16 = 0x1A38;
/// RGB565 white on the selection background.
pub const SEL_FG: u16 = 0xFFFF;
/// Alias for the primary accent. Widgets that just want "the accent"
/// use this; the rest pick CYAN/MAGENTA explicitly.
pub const ACCENT: u16 = CYAN;
/// RGB565 red. Error toasts / failure markers.
pub const ERR: u16 = 0xF800;
/// RGB565 green (R=0, G=63, B=0). Success markers.
pub const OK: u16 = 0x07E0;

/// One builtin palette command. `name` is the fuzzy target; `desc` is a
/// short human label shown in the picker; `key` is the recommended single-
/// key shortcut rendered next to the entry (purely advisory in M1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command {
    pub name: &'static str,
    pub desc: &'static str,
    pub key: char,
}

/// The 12 builtin commands rendered in the M1 palette picker. Order is the
/// display order in the (empty-query) listing.
pub const BUILTINS: [Command; 12] = [
    Command {
        name: "connect",
        desc: "Open profile connect picker",
        key: 'c',
    },
    Command {
        name: "disconnect",
        desc: "Drop the current SSH session",
        key: 'd',
    },
    Command {
        name: "theme",
        desc: "Open the theme editor",
        key: 't',
    },
    Command {
        name: "voice",
        desc: "Open the voice menu",
        key: 'v',
    },
    Command {
        name: "palette",
        desc: "Reopen the command palette",
        key: 'p',
    },
    Command {
        name: "profiles",
        desc: "Browse all known profiles",
        key: 'a',
    },
    Command {
        name: "handoff",
        desc: "Open a saved handoff note",
        key: 'h',
    },
    Command {
        name: "memory",
        desc: "Search the obsidian vault",
        key: 'm',
    },
    Command {
        name: "help",
        desc: "Show the hotkey overlay",
        key: '?',
    },
    Command {
        name: "doctor",
        desc: "Run m5tui doctor self-check",
        key: 'D',
    },
    Command {
        name: "settings",
        desc: "Open device settings",
        key: 's',
    },
    Command {
        name: "quit",
        desc: "Exit m5Tui",
        key: 'q',
    },
];

/// Score `query` against `name`. The function is pure and allocation-free.
///
/// - Empty `query` returns `0` (no filtering — the caller treats the empty
///   query as "show everything").
/// - Exact match: + the difference of the lengths (longer `name` scores
///   higher when the query equals the full name).
/// - Case-sensitive prefix: +`10` when `name` starts with `query`.
/// - Case-insensitive prefix: +`5` when `name` lowercased starts with the
///   query lowercased (stacks on top of the case-sensitive bonus when both
///   apply).
/// - Subsequence: +`2 * count_of_query_chars_in_order`.
///
/// Then subtract a gap penalty of `1 * (name_len - query_len)` for the
/// characters in `name` that are not part of the query. The final score is
/// clamped to `0`.
pub fn fuzzy_score(query: &str, name: &str) -> i32 {
    if query.is_empty() {
        return 0;
    }

    let q = query.as_bytes();
    let n = name.as_bytes();
    let qlen = q.len();
    let nlen = n.len();

    if qlen > nlen {
        // Query longer than the candidate: only case-insensitive prefix or
        // subsequence can score. We still walk it for the subsequence bonus.
    }

    let mut score: i32 = 0;

    // Exact match bonus: len(name) - len(query) when query is a prefix of
    // name. The bigger the name, the stronger the match.
    if nlen >= qlen && &n[..qlen] == q {
        score += (nlen - qlen) as i32;
    }

    // Case-insensitive prefix bonus.
    if nlen >= qlen {
        let mut ci = true;
        for i in 0..qlen {
            if !n[i].eq_ignore_ascii_case(&q[i]) {
                ci = false;
                break;
            }
        }
        if ci {
            score += 5;
        }
    }

    // Case-sensitive prefix bonus (stacks on the case-insensitive one when
    // both apply, i.e. when the cases actually match).
    if nlen >= qlen && &n[..qlen] == q {
        score += 10;
    }

    // Subsequence bonus: count of query chars that appear in `name` in
    // order. Each matching step adds 2.
    let mut subseq = 0usize;
    let mut ni = 0usize;
    for &qb in q {
        let mut found = false;
        while ni < nlen {
            if n[ni].eq_ignore_ascii_case(&qb) {
                subseq += 1;
                ni += 1;
                found = true;
                break;
            }
            ni += 1;
        }
        if !found {
            break;
        }
    }
    score += (subseq as i32) * 2;

    // Gap penalty: characters in `name` not consumed by the query.
    if nlen > qlen {
        score -= (nlen - qlen) as i32;
    }

    if score < 0 {
        0
    } else {
        score
    }
}

/// Run `fuzzy_score` over every command and return `(index, score)` pairs
/// with `score > 0`, sorted by score descending (ties broken by `index`
/// ascending so the listing is stable).
pub fn filter(query: &str, cmds: &[Command]) -> Vec<(usize, i32)> {
    let mut out: Vec<(usize, i32)> = cmds
        .iter()
        .enumerate()
        .map(|(i, c)| (i, fuzzy_score(query, c.name)))
        .filter(|(_, s)| *s > 0)
        .collect();
    out.sort_by(|(ai, as_), (bi, bs)| bs.cmp(as_).then(ai.cmp(bi)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_returns_zero() {
        assert_eq!(fuzzy_score("", "connect"), 0);
    }

    #[test]
    fn single_char_prefix_is_positive() {
        let s = fuzzy_score("c", "connect");
        assert!(s > 0, "expected positive score, got {s}");
    }

    #[test]
    fn no_overlap_returns_zero() {
        assert_eq!(fuzzy_score("xyz", "connect"), 0);
    }

    #[test]
    fn subsequence_match_is_positive() {
        assert!(fuzzy_score("cnn", "connect") > 0);
    }

    #[test]
    fn case_insensitive_query_is_positive() {
        assert!(fuzzy_score("CON", "connect") > 0);
    }

    #[test]
    fn filter_c_returns_at_least_one_match() {
        let hits = filter("c", &BUILTINS);
        assert!(!hits.is_empty());
    }

    #[test]
    fn filter_sorts_descending() {
        let hits = filter("c", &BUILTINS);
        for w in hits.windows(2) {
            assert!(
                w[0].1 >= w[1].1,
                "filter result not sorted desc: {:?}",
                hits
            );
        }
    }

    #[test]
    fn filter_exact_name_scores_higher_than_subsequence() {
        let exact = fuzzy_score("connect", "connect");
        let sub = fuzzy_score("cnn", "connect");
        assert!(exact > sub, "exact={exact} subseq={sub}");
    }

    #[test]
    fn filter_keeps_only_positive_scores() {
        let hits = filter("zzz", &BUILTINS);
        assert!(hits.is_empty());
    }
}
