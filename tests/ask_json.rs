use assert_cmd::prelude::*;
use predicates::str::contains;
use std::process::Command;

#[test]
fn ask_json_mock() {
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(["ask", "--provider", "mock", "What", "time", "is", "it?", "--json"]);
    cmd.assert().success().stdout(contains("\"answer\""));
}
