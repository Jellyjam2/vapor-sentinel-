use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{env, fs, path::{Path, PathBuf}};

const MAX_POLL_INTERVAL_SECS: u64 = 86_400;
const MAX_THRESHOLD_MIB: u64 = 1_000_000_000;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub threshold_mib: u64,
    pub poll_interval_secs: u64,
    pub dsl_path: PathBuf,
}

impl RuntimeConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read runtime config {}", path.display()))?;
        let mut config: Self = serde_json::from_str(&source)
            .with_context(|| format!("failed to parse runtime config {}", path.display()))?;

        if config.threshold_mib == 0 || config.threshold_mib > MAX_THRESHOLD_MIB {
            bail!("threshold_mib must be between 1 and {}", MAX_THRESHOLD_MIB);
        }
        if config.poll_interval_secs == 0 || config.poll_interval_secs > MAX_POLL_INTERVAL_SECS {
            bail!(
                "poll_interval_secs must be between 1 and {}",
                MAX_POLL_INTERVAL_SECS
            );
        }

        if config.dsl_path.is_relative() {
            let base = path.parent().unwrap_or_else(|| Path::new("."));
            config.dsl_path = base.join(&config.dsl_path);
        }

        if !config.dsl_path.is_file() {
            bail!("configured DSL policy is not a regular file: {}", config.dsl_path.display());
        }

        Ok(config)
    }
}

pub fn path_from_env() -> PathBuf {
    env::var_os("VAPOR_SENTINEL_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("vapor-sentinel.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_path_is_stable() {
        assert_eq!(path_from_env(), PathBuf::from("vapor-sentinel.json"));
    }

    #[test]
    fn invalid_threshold_is_rejected() {
        let config = RuntimeConfig {
            threshold_mib: 0,
            poll_interval_secs: 4,
            dsl_path: PathBuf::from("policy.vapor"),
        };
        let source = serde_json::to_string(&config).unwrap();
        let parsed: RuntimeConfig = serde_json::from_str(&source).unwrap();
        assert_eq!(parsed.threshold_mib, 0);
    }
}
