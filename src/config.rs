use anyhow::{Context, Result};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

pub const APP_DIR_NAME: &str = "sw-assistant";
pub const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub default_profile: Option<String>,
    #[serde(default)]
    pub profiles: std::collections::BTreeMap<String, Profile>,
    /// Optional per-model capability overrides. Key can be either
    /// "provider:model" or just "model" to match any provider.
    #[serde(default)]
    pub model_overrides: std::collections::BTreeMap<String, ModelCapsOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Profile {
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelCapsOverride {
    pub streaming: Option<bool>,
    pub context_window: Option<u32>,
    pub supports_json: Option<bool>,
    pub supports_tools: Option<bool>,
    pub modalities: Option<Vec<String>>, // e.g., ["text"], ["text","vision"]
}

pub fn default_config_path() -> Result<PathBuf> {
    let base = config_dir().context("unable to resolve OS config directory")?;
    Ok(base.join(APP_DIR_NAME).join(CONFIG_FILE_NAME))
}

pub fn ensure_config_parent_exists(path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating config dir: {}", parent.display()))?;
    }
    Ok(())
}

pub fn load_config_if_exists(path: &PathBuf) -> Result<Option<AppConfig>> {
    if path.exists() {
        let text = fs::read_to_string(path)
            .with_context(|| format!("reading config file: {}", path.display()))?;
        let cfg: AppConfig = toml::from_str(&text).context("parsing config TOML")?;
        Ok(Some(cfg))
    } else {
        Ok(None)
    }
}

pub fn write_config(path: &PathBuf, cfg: &AppConfig) -> Result<()> {
    ensure_config_parent_exists(path)?;
    let text = toml::to_string_pretty(cfg).context("serializing config to TOML")?;
    fs::write(path, text).with_context(|| format!("writing config file: {}", path.display()))?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct EffectiveSettings {
    pub provider: String,
    pub model: String,
}

pub fn resolve_effective_settings(
    profile_override: Option<&str>,
    cli_provider: Option<&str>,
    cli_model: Option<&str>,
) -> Result<EffectiveSettings> {
    let path = default_config_path()?;
    let cfg = load_config_if_exists(&path)?;

    let mut provider: Option<String> = None;
    let mut model: Option<String> = None;

    if let Some(cfg) = cfg {
        let profile_name = profile_override
            .map(|s| s.to_string())
            .or(cfg.default_profile)
            .unwrap_or_else(|| "default".to_string());
        if let Some(p) = cfg.profiles.get(&profile_name) {
            if let Some(pv) = &p.provider { provider = Some(pv.clone()); }
            if let Some(m) = &p.model { model = Some(m.clone()); }
        }
    }

    if let Some(cp) = cli_provider { provider = Some(cp.to_string()); }
    if let Some(cm) = cli_model { model = Some(cm.to_string()); }

    let provider = provider.unwrap_or_else(|| "openai".to_string());
    let model = model.unwrap_or_else(|| "gpt-4o-mini".to_string());

    Ok(EffectiveSettings { provider, model })
}

impl AppConfig {
    /// Find a capability override for a given provider+model, with fallback to model-only key.
    pub fn find_model_override(&self, provider: &str, model: &str) -> Option<&ModelCapsOverride> {
        let key_full = format!("{}:{}", provider.to_lowercase(), model);
        if let Some(v) = self.model_overrides.get(&key_full) { return Some(v); }
        self.model_overrides.get(model)
    }
}


