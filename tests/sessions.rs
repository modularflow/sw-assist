use assert_cmd::prelude::*;
use std::process::Command;
use std::fs;

fn temp_dirs() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn session_new_list_switch_show() {
    let temp = temp_dirs();
    let xdg_data_home = temp.path().join(".local/share");
    fs::create_dir_all(&xdg_data_home).unwrap();

    // new NAME
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("XDG_DATA_HOME", &xdg_data_home)
        .args(["session", "new", "s1"]);
    cmd.assert().success();

    // list
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("XDG_DATA_HOME", &xdg_data_home)
        .args(["session", "list"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("s1"));

    // switch
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("XDG_DATA_HOME", &xdg_data_home)
        .args(["session", "switch", "s1"]);
    cmd.assert().success();

    // show
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("XDG_DATA_HOME", &xdg_data_home)
        .args(["session", "show"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("active: s1"));

    // JSON list
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("XDG_DATA_HOME", &xdg_data_home)
        .args(["session", "list", "--json"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("\"name\":"));
}

#[test]
fn ask_appends_to_session() {
    let temp = temp_dirs();
    let xdg_data_home = temp.path().join(".local/share");
    let xdg_config_home = temp.path().join(".config");
    fs::create_dir_all(&xdg_data_home).unwrap();
    fs::create_dir_all(&xdg_config_home.join("sw-assistant")).unwrap();
    fs::write(
        xdg_config_home.join("sw-assistant").join("config.toml"),
        "default_profile = \"default\"\n\n[profiles.default]\nprovider = \"mock\"\nmodel = \"test-model\"\n",
    ).unwrap();

    // run ask with session
    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("XDG_DATA_HOME", &xdg_data_home)
        .env("XDG_CONFIG_HOME", &xdg_config_home)
        .args(["ask", "--session", "s1", "--provider", "mock", "hi"]);
    cmd.assert().success();

    // verify session file has two lines
    let session_file = xdg_data_home.join("sw-assistant").join("sessions").join("s1.jsonl");
    let content = fs::read_to_string(session_file).unwrap();
    let lines: Vec<_> = content.lines().collect();
    assert_eq!(lines.len(), 2);
}

// truncation tested within module unit test in src/session.rs
