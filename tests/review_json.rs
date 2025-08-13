use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;

#[test]
fn review_json_offline_shape() {
    // create a fake diff file
    let temp = tempfile::tempdir().unwrap();
    let diff_path = temp.path().join("changes.diff");
    fs::write(&diff_path, "--- a/foo\n+++ b/foo\n@@\n-line\n+line2\n").unwrap();

    // no provider/env; uses offline mock JSON
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(["review", "--diff-file", diff_path.to_str().unwrap(), "--json"]);
    let output = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&output);

    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    let feedback = v.get("feedback").expect("has feedback");
    for key in ["correctness", "style", "security", "tests", "suggestions"] { 
        let arr = feedback.get(key).expect("has key").as_array().expect("array");
        if key == "suggestions" { assert!(!arr.is_empty()); }
    }
}
