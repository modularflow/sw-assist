use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;

fn net_enabled() -> bool {
    std::env::var("RUN_NET_TESTS").ok().as_deref() == Some("1")
}

#[test]
fn todos_normalize_groq_json() {
    if !net_enabled() { return; }
    if std::env::var("GROQ_API_KEY").is_err() { return; }
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("todos.txt");
    fs::write(&file, "// TODO: write docs @doc\n// FIXME: null crash @qa\n").unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("GROQ_API_KEY", std::env::var("GROQ_API_KEY").unwrap())
        .args(["todos", "--file", file.to_str().unwrap(), "--provider", "groq", "--normalize", "--json"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid json array");
    assert!(v.as_array().is_some());
}


