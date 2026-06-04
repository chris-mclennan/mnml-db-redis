//! Config file at `~/.config/mnml-db-redis.toml`.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_row_limit")]
    pub row_limit: u32,
    #[serde(default)]
    pub connections: Vec<Connection>,
}

fn default_row_limit() -> u32 {
    500
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub name: String,
    /// Redis URL — `redis://[:password@]host:port[/db]` or
    /// `rediss://` for TLS. `${ENV_VAR}` expansion at load time.
    pub url: String,
}

impl Config {
    pub const EXAMPLE: &'static str = r##"# mnml-db-redis config. Edit and re-run.
# chmod 600 the file — passwords live in the URL.

row_limit = 500

[[connections]]
name = "local"
url = "redis://localhost:6379"

# [[connections]]
# name = "prod-cache"
# url = "rediss://:${PROD_REDIS_PASS}@redis.prod.example.com:6380/0"

# [[connections]]
# name = "elasticache"
# url = "redis://my-cluster.abc.use1.cache.amazonaws.com:6379"
"##;

    pub fn validate(&self) -> Result<()> {
        if self.connections.is_empty() {
            return Err(anyhow!(
                "config: at least one [[connections]] entry required"
            ));
        }
        if self.row_limit == 0 {
            return Err(anyhow!("config: row_limit must be > 0"));
        }
        for (i, c) in self.connections.iter().enumerate() {
            if c.name.trim().is_empty() {
                return Err(anyhow!("connection #{i}: `name` is required"));
            }
            if c.url.trim().is_empty() {
                return Err(anyhow!("connection #{i} ({}): `url` is required", c.name));
            }
        }
        Ok(())
    }

    pub fn expand_env(&mut self) {
        for c in self.connections.iter_mut() {
            c.url = expand_env(&c.url);
        }
    }
}

fn expand_env(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'{') {
            chars.next();
            let mut name = String::new();
            for c in chars.by_ref() {
                if c == '}' {
                    break;
                }
                name.push(c);
            }
            match std::env::var(&name) {
                Ok(v) => out.push_str(&v),
                Err(_) => {
                    out.push_str("${");
                    out.push_str(&name);
                    out.push('}');
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("mnml-db-redis.toml")
}

pub fn load() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, Config::EXAMPLE)?;
        return Err(anyhow!(
            "wrote config template to {} — edit it (chmod 600!) then re-run",
            path.display()
        ));
    }
    let text = std::fs::read_to_string(&path)?;
    let mut cfg: Config = toml::from_str(&text)?;
    cfg.validate()?;
    cfg.expand_env();
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_config_parses_and_validates() {
        let cfg: Config = toml::from_str(Config::EXAMPLE).unwrap();
        cfg.validate().unwrap();
        assert!(!cfg.connections.is_empty());
    }

    #[test]
    fn validate_rejects_empty_connections() {
        let cfg: Config = toml::from_str("row_limit = 100").unwrap();
        assert!(cfg.validate().is_err());
    }
}
