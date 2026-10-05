//! Bounded, deployment-controlled configuration and policy file loading.
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
pub enum Metric {
    #[serde(rename = "SYSTEM_USED_MEMORY_MIB")]
    UsedMemoryMib,
    #[default]
    #[serde(rename = "SYSTEM_AVAILABLE_MEMORY_PERCENT")]
    AvailableMemoryPercent,
}
impl Metric {
    pub fn name(self) -> &'static str {
        match self {
            Self::UsedMemoryMib => "SYSTEM_USED_MEMORY_MIB",
            Self::AvailableMemoryPercent => "SYSTEM_AVAILABLE_MEMORY_PERCENT",
        }
    }
    pub fn unit(self) -> &'static str {
        match self {
            Self::UsedMemoryMib => "MiB",
            Self::AvailableMemoryPercent => "percent",
        }
    }
}
fn default_repeat() -> u64 {
    300
}
fn default_retry() -> u64 {
    30
}
fn default_recovery() -> u32 {
    2
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub metric: Metric,
    pub poll_interval_secs: u64,
    pub dsl_path: PathBuf,
    #[serde(default = "default_repeat")]
    pub notification_repeat_secs: u64,
    #[serde(default = "default_retry")]
    pub notification_retry_secs: u64,
    #[serde(default = "default_recovery")]
    pub recovery_samples: u32,
}

pub fn read_bounded(path: &Path, max_bytes: usize) -> Result<String> {
    let file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    if !file.metadata()?.is_file() {
        bail!("expected regular file: {}", path.display());
    }
    let mut bytes = Vec::new();
    file.take(max_bytes as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > max_bytes {
        bail!("{} exceeds {max_bytes} bytes", path.display());
    }
    String::from_utf8(bytes).context("input must be UTF-8")
}

impl RuntimeConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let source = read_bounded(path, 16 * 1024)?;
        let mut config: Self = serde_json::from_str(&source).with_context(|| {
            format!(
                "invalid config {} (put threshold comparisons in the DSL, not threshold_mib)",
                path.display()
            )
        })?;
        for (name, value) in [
            ("poll_interval_secs", config.poll_interval_secs),
            ("notification_repeat_secs", config.notification_repeat_secs),
            ("notification_retry_secs", config.notification_retry_secs),
        ] {
            if !(1..=86_400).contains(&value) {
                bail!("{name} must be between 1 and 86400");
            }
        }
        if !(1..=100).contains(&config.recovery_samples) {
            bail!("recovery_samples must be between 1 and 100");
        }
        if config.dsl_path.is_relative() {
            config.dsl_path = path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(&config.dsl_path);
        }
        if !config.dsl_path.is_file() {
            bail!(
                "configured DSL policy is not a regular file: {}",
                config.dsl_path.display()
            );
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
    fn load(value: serde_json::Value) -> Result<RuntimeConfig> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("policy.vapor"), "vapor s(){}")?;
        let path = dir.path().join("config.json");
        std::fs::write(&path, value.to_string())?;
        RuntimeConfig::load(&path)
    }
    fn valid() -> serde_json::Value {
        serde_json::json!({"poll_interval_secs": 4, "dsl_path": "policy.vapor"})
    }
    #[test]
    fn relative_policy_resolves_against_config() {
        let config = load(valid()).unwrap();
        assert!(config.dsl_path.is_absolute());
        assert_eq!(config.notification_repeat_secs, 300);
    }
    #[test]
    fn invalid_intervals_and_recovery_are_rejected_by_loader() {
        for key in [
            "poll_interval_secs",
            "notification_repeat_secs",
            "notification_retry_secs",
            "recovery_samples",
        ] {
            for value in [0, 100_000] {
                let mut v = valid();
                v[key] = value.into();
                assert!(load(v).is_err(), "{key}={value}");
            }
        }
    }
    #[test]
    fn unknown_keys_and_metrics_fail_at_startup() {
        let mut v = valid();
        v["threshold_mib"] = 100.into();
        assert!(load(v).is_err());
        let mut v = valid();
        v["metric"] = "TYPO".into();
        assert!(load(v).is_err());
    }
    #[test]
    fn missing_policy_is_rejected() {
        let mut v = valid();
        v["dsl_path"] = "missing.vapor".into();
        assert!(load(v).is_err());
    }
    #[test]
    fn bounded_reader_rejects_oversized_and_malformed_input() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("input");
        std::fs::write(&path, vec![b'x'; 20]).unwrap();
        assert!(read_bounded(&path, 19).is_err());
        assert_eq!(read_bounded(&path, 20).unwrap().len(), 20);
        std::fs::write(&path, [255]).unwrap();
        assert!(read_bounded(&path, 20).is_err());
        std::fs::write(&path, "{").unwrap();
        assert!(RuntimeConfig::load(&path).is_err());
    }
}
