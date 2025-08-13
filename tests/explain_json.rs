use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;

#[test]
fn explain_mock_json_schema() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("code.rs");
    fs::write(&file, "fn main(){}\nfn other(){}\n").unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(["explain", "--file", file.to_str().unwrap(), "--provider", "mock", "--json"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    assert!(v.get("model").is_some());
    assert!(v.get("file").is_some());
    assert!(v.get("range").is_some());
    assert!(v.get("explanation").is_some());
}
