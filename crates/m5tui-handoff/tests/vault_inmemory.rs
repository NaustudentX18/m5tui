//! Tests for `InMemoryVaultClient`. Exercises the JSONL constructor
//! plus the trait-level `query` round-trip.

use m5tui_handoff::{HandoffError, Hit, InMemoryVaultClient, VaultSearch};

#[test]
fn from_jsonl_with_two_hits_serves_both_for_any_query() {
    let jsonl = "\
{\"path\":\"vault/a.md\",\"title\":\"A\",\"snippet\":\"alpha\"}
{\"path\":\"vault/b.md\",\"title\":\"B\",\"snippet\":\"beta\"}
";
    let client =
        InMemoryVaultClient::from_jsonl(jsonl).expect("two well-formed lines should parse");
    assert_eq!(client.len(), 2);

    let hits = client
        .query("anything")
        .expect("query should succeed for a valid in-memory client");
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].path, "vault/a.md");
    assert_eq!(hits[1].path, "vault/b.md");
}

#[test]
fn from_jsonl_with_malformed_line_returns_err() {
    let jsonl = "{\"path\":\"vault/a.md\",\"title\":\"A\",\"snippet\":\"alpha\"}\nnot json\n";
    let result = InMemoryVaultClient::from_jsonl(jsonl);
    assert!(
        matches!(result, Err(HandoffError::Parse(_))),
        "malformed JSONL should return HandoffError::Parse, got {result:?}"
    );
}

#[test]
fn empty_client_returns_empty_vec_for_any_query() {
    let client = InMemoryVaultClient::new(Vec::new());
    assert!(client.is_empty());
    let hits = client
        .query("anything")
        .expect("empty client should still return Ok");
    assert!(hits.is_empty(), "empty client must serve no hits");
}

#[test]
fn new_with_explicit_corpus_is_queryable() {
    let hits = vec![Hit {
        path: "vault/x.md".to_string(),
        title: "X".to_string(),
        snippet: "snippet".to_string(),
    }];
    let client = InMemoryVaultClient::new(hits);
    let out = client.query("nope").expect("query ok");
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].title, "X");
}
