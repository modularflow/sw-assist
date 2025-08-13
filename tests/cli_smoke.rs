use assert_cmd::prelude::*;
use predicates::str::contains;
use std::process::Command;

#[test]
fn prints_help() {
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.arg("--help");
    cmd.assert().success().stdout(contains("CLI AI software assistant"));
}

#[test]
fn ask_requires_prompt() {
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.arg("ask");
    let assert = cmd.assert().failure();
    // clap should error about missing argument
    assert.stderr(contains("Usage:"));
}

#[test]
fn ask_stub_works() {
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(["ask", "--provider", "mock", "What", "is", "Rust?"]);
    cmd.assert().success().stdout(contains("stub answer"));
}


