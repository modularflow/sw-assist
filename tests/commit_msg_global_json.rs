use assert_cmd::prelude::*;
use predicates::str::contains;
use std::fs;
use std::process::Command;

#[test]
fn commit_msg_honors_global_json() {
    let temp = tempfile::tempdir().unwrap();
    let diff_path = temp.path().join("d.diff");
    fs::write(&diff_path, "--- a/foo\n+++ b/foo\n@@\n-line\n+line2\n").unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(["--json", "commit-msg", "--diff-file", diff_path.to_str().unwrap()]);
    cmd.assert().success().stdout(contains("\"type\""));
}
