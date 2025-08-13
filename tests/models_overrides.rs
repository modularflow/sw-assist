use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;

#[test]
fn models_list_applies_overrides() {
    let temp = tempfile::tempdir().unwrap();
    let xdg_config_home = temp.path().join(".config");
    let xdg_cache_home = temp.path().join(".cache");
    std::fs::create_dir_all(xdg_config_home.join("sw-assistant")).unwrap();
    std::fs::create_dir_all(&xdg_cache_home).unwrap();
    // write config with default profile mock + overrides
    let cfg = r#"
default_profile = "default"

[profiles.default]
provider = "mock"
model = "mock-small"

[model_overrides]
"mock:mock-small" = { supports_json = false, supports_tools = true, modalities = ["text","vision"] }
"#;
    fs::write(xdg_config_home.join("sw-assistant").join("config.toml"), cfg).unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("XDG_CONFIG_HOME", &xdg_config_home)
        .env("XDG_CACHE_HOME", &xdg_cache_home)
        .args(["models", "list", "--provider", "mock", "--json"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);

    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = v.as_array().unwrap();
    let first = arr.iter().find(|m| m.get("name").and_then(|x| x.as_str()) == Some("mock-small")).unwrap();
    assert_eq!(first.get("supports_json").and_then(|x| x.as_bool()), Some(false));
    assert_eq!(first.get("supports_tools").and_then(|x| x.as_bool()), Some(true));
    let mods = first.get("modalities").and_then(|x| x.as_array()).unwrap();
    assert!(mods.iter().any(|x| x.as_str() == Some("vision")));
}
