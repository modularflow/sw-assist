use assert_cmd::prelude::*;
use predicates::str::contains;
use std::fs;
use std::process::Command;

#[test]
fn summarize_works_with_mock() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("f.txt");
    fs::write(&file, "Line one\nLine two\nLine three").unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(["summarize", "--file", file.to_str().unwrap(), "--provider", "mock", "--max-tokens", "10"]);
    cmd.assert().success();
}

#[test]
fn explain_works_with_range() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("main.rs");
    fs::write(&file, "fn main() {}\nfn other() {}\n").unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(["explain", "--file", file.to_str().unwrap(), "--range", "1:1", "--provider", "mock",]);
    cmd.assert().success();
}
