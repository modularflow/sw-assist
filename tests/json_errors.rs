use assert_cmd::prelude::*;
use predicates::str::contains;
use std::fs;
use std::process::Command;

fn assert_json_error(assert: &assert_cmd::assert::Assert) {
    let out = assert.get_output();
    assert_eq!(out.status.success(), false);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid json error");
    assert!(v.get("code").is_some());
    assert!(v.get("message").is_some());
}

#[test]
fn summarize_missing_file_json_error() {
    let mut cmd = Command::cargo_bin("sw").unwrap();
    let assert = cmd.args(["summarize", "--file", "missing.txt", "--json"]).assert();
    assert_json_error(&assert);
}

#[test]
fn explain_missing_file_json_error() {
    let mut cmd = Command::cargo_bin("sw").unwrap();
    let assert = cmd.args(["explain", "--file", "missing.rs", "--json"]).assert();
    assert_json_error(&assert);
}

#[test]
fn explain_invalid_range_json_error() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("f.rs");
    fs::write(&file, "fn main(){}\n").unwrap();
    let mut cmd = Command::cargo_bin("sw").unwrap();
    let assert = cmd.args(["explain", "--file", file.to_str().unwrap(), "--range", "bad", "--json"]).assert();
    assert_json_error(&assert);
}

#[test]
fn review_empty_diff_json_error() {
    let temp = tempfile::tempdir().unwrap();
    let diff = temp.path().join("empty.diff");
    fs::write(&diff, "").unwrap();
    let mut cmd = Command::cargo_bin("sw").unwrap();
    let assert = cmd.args(["review", "--diff-file", diff.to_str().unwrap(), "--json"]).assert();
    assert_json_error(&assert);
}

#[test]
fn commit_msg_missing_diff_json_error() {
    let mut cmd = Command::cargo_bin("sw").unwrap();
    let assert = cmd.args(["commit-msg", "--diff-file", "missing.diff", "--json"]).assert();
    assert_json_error(&assert);
}

#[test]
fn session_switch_unknown_json_error() {
    let mut cmd = Command::cargo_bin("sw").unwrap();
    let assert = cmd.args(["session", "switch", "unknown", "--json"]).assert();
    assert_json_error(&assert);
}
