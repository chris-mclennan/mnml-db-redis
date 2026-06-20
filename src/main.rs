mod app;
mod blit;
mod config;
mod keys;
mod redis_client;
mod theme;
mod ui;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "mnml-db-redis",
    version,
    about = "Redis command playground for mnml"
)]
struct Cli {
    #[arg(long)]
    check: bool,
    #[arg(long, value_name = "SOCKET")]
    blit: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = config::load()?;
    if cli.check {
        println!("config: {}", config::config_path().display());
        println!("row_limit: {}", cfg.row_limit);
        for (i, c) in cfg.connections.iter().enumerate() {
            println!("  connection {} ({}): {}", i + 1, c.name, scrub_url(&c.url));
        }
        return Ok(());
    }
    let mut app = app::App::new(cfg).await?;
    if let Some(socket) = cli.blit {
        blit::run(&mut app, std::path::Path::new(&socket)).await
    } else {
        ui::run(&mut app).await
    }
}

/// Redact `:<pass>@` in a redis:// URL for terminal display.
fn scrub_url(url: &str) -> String {
    let Some(scheme_end) = url.find("://") else {
        return url.to_string();
    };
    let rest = &url[scheme_end + 3..];
    let Some(at) = rest.find('@') else {
        return url.to_string();
    };
    let userinfo = &rest[..at];
    let Some(colon) = userinfo.find(':') else {
        return url.to_string();
    };
    let user = &userinfo[..colon];
    let prefix = &url[..scheme_end + 3];
    let suffix = &rest[at..];
    format!("{prefix}{user}:****{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrub_url_hides_password() {
        let s = scrub_url("redis://:hunter2@redis.example.com:6379");
        assert_eq!(s, "redis://:****@redis.example.com:6379");
    }

    #[test]
    fn scrub_url_no_pass_idempotent() {
        let s = scrub_url("redis://localhost:6379");
        assert_eq!(s, "redis://localhost:6379");
    }
}
