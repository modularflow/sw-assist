use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;

#[test]
fn session_search_json() {
    let temp = tempfile::tempdir().unwrap();
    let xdg_data_home = temp.path().join(".local/share");
    fs::create_dir_all(xdg_data_home.join("sw-assistant").join("sessions")).unwrap();
    let file = xdg_data_home.join("sw-assistant").join("sessions").join("s1.jsonl");
    fs::write(&file, format!("{}\n{}\n",
        serde_json::json!({"timestamp_ms": 1, "role":"user","content":"hello world","model":null,"usage":null}),
        serde_json::json!({"timestamp_ms": 2, "role":"assistant","content":"[stub answer] hello world","model":"m","usage":null})
    )).unwrap();

    // search for 'hello'
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("XDG_DATA_HOME", &xdg_data_home)
        .args(["session", "search", "s1", "--contains", "hello", "--json"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("hello world"));
}
