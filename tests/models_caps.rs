use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn models_list_json_includes_capabilities() {
    let temp = tempfile::tempdir().unwrap();
    let xdg_cache_home = temp.path().join(".cache");
    std::fs::create_dir_all(&xdg_cache_home).unwrap();

    let mut cmd = Command::cargo_bin("sw").unwrap();
    cmd.env("XDG_CACHE_HOME", &xdg_cache_home)
        .args(["models", "list", "--provider", "mock", "--json"]);
    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8_lossy(&out);

    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    let arr = v.as_array().expect("array of models");
    assert!(!arr.is_empty(), "should return at least one model");
    for m in arr {
        assert!(m.get("name").and_then(|x| x.as_str()).is_some());
        assert_eq!(m.get("provider").and_then(|x| x.as_str()), Some("mock"));
        assert!(m.get("source").and_then(|x| x.as_str()).is_some());
        assert!(m.get("streaming").and_then(|x| x.as_bool()).is_some());
        // context_window can be null or number
        assert!(m.get("context_window").is_some());
        assert!(m.get("supports_json").and_then(|x| x.as_bool()).is_some());
        assert!(m.get("supports_tools").and_then(|x| x.as_bool()).is_some());
        let mods = m.get("modalities").and_then(|x| x.as_array()).expect("modalities array");
        assert!(!mods.is_empty());
    }
}
