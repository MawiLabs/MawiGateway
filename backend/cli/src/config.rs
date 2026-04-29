//! Persistent CLI config at `~/.mg/config.yaml`. Stores the gateway
//! URL and (optionally) an API key. The file is created with mode 0600
//! so other users on the box can't read it. Format is YAML so humans
//! can edit it; YAML serializer is forgiving about missing fields.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoredConfig {
    pub gateway_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

pub fn config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("could not resolve $HOME"))?;
    Ok(home.join(".mg").join("config.yaml"))
}

pub fn load() -> Result<StoredConfig> {
    let path = config_path()?;
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let cfg: StoredConfig = serde_yaml::from_str(&text)
        .with_context(|| format!("parse {}", path.display()))?;
    Ok(cfg)
}

pub fn save(cfg: &StoredConfig) -> Result<()> {
    let path = config_path()?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    }
    let yaml = serde_yaml::to_string(cfg).context("serialize config")?;
    std::fs::write(&path, yaml).with_context(|| format!("write {}", path.display()))?;
    // Tighten permissions on Unix so the API key isn't world-readable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(())
}

pub fn clear() -> Result<()> {
    let path = config_path()?;
    if path.exists() {
        std::fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
    }
    Ok(())
}
