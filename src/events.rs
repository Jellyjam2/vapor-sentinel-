//! Versioned JSONL output; the same evaluation metadata accompanies webhooks.
use anyhow::{Context, Result};
use serde::Serialize;
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn unix_ms() -> Result<u64> {
    u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())
        .context("timestamp overflow")
}
#[derive(Clone, Serialize)]
pub struct RunContext {
    pub schema_version: u32,
    pub run_id: String,
    pub source_id: String,
    pub policy_sha256: String,
}
impl RunContext {
    pub fn new(policy: &str, replay: bool) -> Result<Self> {
        use sha2::{Digest, Sha256};
        let source_id = if replay {
            "replay".into()
        } else {
            std::env::var("VAPOR_SENTINEL_SOURCE_ID")
                .or_else(|_| std::env::var("HOSTNAME"))
                .or_else(|_| std::env::var("COMPUTERNAME"))
                .unwrap_or_else(|_| "local-host".into())
        };
        if source_id.is_empty() || source_id.len() > 128 || source_id.chars().any(char::is_control)
        {
            anyhow::bail!("source identity must be 1..128 bytes without control characters");
        }
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        Ok(Self {
            schema_version: 1,
            run_id: format!("{nanos}-{}", std::process::id()),
            source_id,
            policy_sha256: format!("{:x}", Sha256::digest(policy.as_bytes())),
        })
    }
}

pub struct EventWriter {
    file: Option<File>,
}
impl EventWriter {
    /// Append without truncating existing evidence. Rotation and retention are
    /// deployment responsibilities; stderr never mixes with the JSONL stream.
    pub fn new(path: Option<&Path>) -> Result<Self> {
        let file = path
            .map(|p| {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(p)
                    .with_context(|| format!("cannot append evidence {}", p.display()))
            })
            .transpose()?;
        Ok(Self { file })
    }
    pub fn emit<T: Serialize>(&mut self, value: &T) -> Result<()> {
        let mut line = serde_json::to_vec(value)?;
        line.push(b'\n');
        if let Some(file) = self.file.as_mut() {
            file.write_all(&line)?;
            file.flush()?;
        }
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        stdout.write_all(&line)?;
        stdout.flush()?;
        Ok(())
    }
}
