use assert_cmd::prelude::*;
use predicates::str::contains;
use std::fs;
use std::process::Command;

#[test]
fn review_mock_text_headings() {
    // create a fake diff file
    let temp = tempfile::tempdir().unwrap();
    let diff_path = temp.path().join("changes.diff");
    fs::write(&diff_path, "--- a/foo\n+++ b/foo\n@@\n-line\n+line2\n").unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(["review", "--diff-file", diff_path.to_str().unwrap()]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Correctness:"));
    assert!(stdout.contains("Style:"));
    assert!(stdout.contains("Security:"));
    assert!(stdout.contains("Tests:"));
    assert!(stdout.contains("Suggestions:"));
}

#[test]
fn commit_msg_mock_json() {
    let temp = tempfile::tempdir().unwrap();
    let diff_path = temp.path().join("d.diff");
    fs::write(&diff_path, "--- a/foo\n+++ b/foo\n@@\n-line\n+line2\n").unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(["commit-msg", "--diff-file", diff_path.to_str().unwrap(), "--json"]);
    cmd.assert().success().stdout(contains("\"type\":"));
}
