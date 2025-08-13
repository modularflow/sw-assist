use assert_cmd::prelude::*;
use predicates::str::contains;
use std::process::Command;

#[test]
fn init_writes_config_to_xdg_config_home() {
    let temp = tempfile::tempdir().unwrap();
    let xdg_config_home = temp.path().join(".config");
    std::fs::create_dir_all(&xdg_config_home).unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("XDG_CONFIG_HOME", &xdg_config_home)
        .args([
            "init",
            "--provider",
            "openai",
            "--api-key",
            "TEST_KEY",
            "--default-model",
            "gpt-5-nano",
            "--profile",
            "default",
        ]);
    cmd.assert().success().stdout(contains("config written:"));

    // Verify file exists
    let cfg_path = xdg_config_home.join("sw-assistant").join("config.toml");
    let contents = std::fs::read_to_string(cfg_path).unwrap();
    assert!(contents.contains("default_profile"));
    assert!(contents.contains("profiles"));
    assert!(contents.contains("openai"));
    assert!(contents.contains("TEST_KEY"));
}

#[test]
fn init_validate_skips_without_keys() {
    // Non-interactive validate should fail gracefully when key is missing, but not hang
    let temp = tempfile::tempdir().unwrap();
    let xdg_config_home = temp.path().join(".config");
    std::fs::create_dir_all(&xdg_config_home).unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    // do not set OPENAI_API_KEY; pass validate flag with provider openai
    let assert = cmd.env("XDG_CONFIG_HOME", &xdg_config_home)
        .args([
            "init",
            "--provider", "openai",
            "--default-model", "gpt-4o-mini",
            "--profile", "default",
            "--validate",
        ]).assert();
    // Should fail (missing key) and not hang
    assert.failure();
}


