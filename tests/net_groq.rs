use assert_cmd::prelude::*;
use std::process::Command;

fn net_disabled() -> bool {
    std::env::var("RUN_NET_TESTS").ok().as_deref() != Some("1")
}

#[test]
fn ask_groq_json() {
    if net_disabled() { return; }
    if std::env::var("GROQ_API_KEY").is_err() { return; }
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("GROQ_API_KEY", std::env::var("GROQ_API_KEY").unwrap())
        .args(["ask", "--provider", "groq", "Hello", "--json"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    assert!(v.get("answer").is_some());
}

#[test]
fn models_list_groq_refresh_json() {
    if net_disabled() { return; }
    if std::env::var("GROQ_API_KEY").is_err() { return; }
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("GROQ_API_KEY", std::env::var("GROQ_API_KEY").unwrap())
        .args(["models", "list", "--provider", "groq", "--refresh", "--json"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);
    let arr: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(arr.as_array().is_some());
}
