use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;

#[test]
fn summarize_mock_json_schema() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("f.txt");
    fs::write(&file, "Line one\nLine two\nLine three").unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(["summarize", "--file", file.to_str().unwrap(), "--provider", "mock", "--json"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    assert!(v.get("model").is_some());
    assert!(v.get("chunks").is_some());
    assert!(v.get("summary").is_some());
}
