use std::{path::PathBuf, time::Duration};

use clap::Args;
use serde::Serialize;

#[derive(Debug, Clone, Args, Serialize)]
pub struct Config {
    #[arg(long, env = "FSP_MCP_HOME", global = true)]
    pub data_dir: Option<PathBuf>,
    #[arg(long, env = "FSP_PROFILE", global = true)]
    pub profile: Option<PathBuf>,
    #[arg(long, default_value = "10000", global = true)]
    pub timeout_ms: u64,
}

impl Config {
    pub fn data_dir(&self) -> PathBuf {
        self.data_dir
            .clone()
            .unwrap_or_else(|| home().join("Library/Application Support/fender-studio-pro-mcp"))
    }

    pub fn profile(&self) -> PathBuf {
        self.profile
            .clone()
            .unwrap_or_else(|| home().join("Library/Application Support/Fender/Studio Pro 8"))
    }

    pub fn mailbox(&self) -> PathBuf {
        self.data_dir().join("mailbox")
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms.clamp(100, 120_000))
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
}
