use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;

#[test]
fn ask_uses_provider_from_config_profile() {
    let temp = tempfile::tempdir().unwrap();
    let xdg_config_home = temp.path().join(".config");
    std::fs::create_dir_all(xdg_config_home.join("sw-assistant")).unwrap();
    let cfg_path = xdg_config_home.join("sw-assistant").join("config.toml");
    fs::write(
        &cfg_path,
        r#"
default_profile = "default"

[profiles.default]
provider = "mock"
model = "gpt-4o-mini"
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("XDG_CONFIG_HOME", &xdg_config_home)
        .args(["ask", "Hello"]);
    cmd.assert().success();
}


