use assert_cmd::prelude::*;
use predicates::str::contains;
use std::fs;
use std::process::Command;

#[test]
fn models_list_mock_and_cache() {
    let temp = tempfile::tempdir().unwrap();
    let xdg_cache_home = temp.path().join(".cache");
    fs::create_dir_all(&xdg_cache_home).unwrap();

    // Run mock listing, should print mock models
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("XDG_CACHE_HOME", &xdg_cache_home)
        .args(["models", "list", "--provider", "mock"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("mock-small"));

    // Ensure cache file is written
    let cache_path = xdg_cache_home.join("sw-assistant").join("models.json");
    assert!(cache_path.exists());

    // Validate TTL and refresh flag (mock path)
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("XDG_CACHE_HOME", &xdg_cache_home)
        .args(["models", "list", "--provider", "mock", "--refresh"]);
    cmd.assert().success().stdout(contains("mock-small"));
}

#[test]
fn todos_scans_text_file() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("file.txt");
    fs::write(&file, "// TODO: fix this @alice [prio:low]\n/* FIXME: edge case @bob [PRIO:HIGH] */\n// note: trivial\n").unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(["todos", "--file", file.to_str().unwrap()]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("TODO"));
    assert!(stdout.contains("FIXME"));

    // JSON
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(["todos", "--file", file.to_str().unwrap(), "--json"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let json = String::from_utf8_lossy(&out);
    assert!(json.contains("\"priority\""));
    assert!(json.contains("@alice"));
}

#[test]
fn todos_json_priority_heuristics() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("todos.txt");
    // Include indicators for high/medium/low
    fs::write(&file,
        "// BUG: crash on null @dev\n\
         // HACK: temporary fix @qa\n\
         // - [ ] polish UI later @pm\n").unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(["todos", "--file", file.to_str().unwrap(), "--json"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json array");
    let arr = v.as_array().expect("array");
    assert_eq!(arr.len(), 3);
    let prios: Vec<_> = arr.iter().map(|e| e.get("priority").and_then(|x| x.as_str()).unwrap_or("").to_string()).collect();
    assert!(prios.contains(&"high".to_string()));
    assert!(prios.contains(&"medium".to_string()));
    assert!(prios.contains(&"low".to_string()));
}

#[test]
fn todos_json_owner_field() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("owners.txt");
    fs::write(&file, "// TODO: something minor @alice\n// FIXME: critical @bob\n").unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.args(["todos", "--file", file.to_str().unwrap(), "--json"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json array");
    let arr = v.as_array().expect("array");
    assert!(arr.iter().any(|e| e.get("owner").and_then(|x| x.as_str()) == Some("@alice")));
    assert!(arr.iter().any(|e| e.get("owner").and_then(|x| x.as_str()) == Some("@bob")));
}

#[test]
fn todos_normalize_with_mock_is_noop_json() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("norm.txt");
    fs::write(&file, "// TODO: refactor module @dev\n").unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    // Using mock provider with --normalize should fall back to regex output
    cmd.args(["todos", "--file", file.to_str().unwrap(), "--provider", "mock", "--normalize", "--json"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json array");
    let arr = v.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    let first = &arr[0];
    assert_eq!(first.get("owner").and_then(|x| x.as_str()), Some("@dev"));
}
