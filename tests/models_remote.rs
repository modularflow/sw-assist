use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn models_remote_optional() {
    if std::env::var("RUN_NET_TESTS").ok().as_deref() != Some("1") { return; }
    if std::env::var("OPENAI_API_KEY").is_err() { return; }
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(["models", "list", "--provider", "openai", "--refresh", "--json"]);
    let assert = cmd.assert();
    let output = assert.get_output();
    assert_eq!(output.status.success(), true);
}
