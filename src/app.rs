//! App state — connection list, active connection, command buffer,
//! last result, status.

use crate::config::{Config, Connection};
use crate::redis_client::{QueryResult, connect, run_command};
use anyhow::Result;
use redis::aio::ConnectionManager;

pub struct App {
    #[allow(dead_code)]
    pub cfg: Config,
    pub connections: Vec<ConnState>,
    pub active: usize,
    pub query: String,
    pub cursor: usize,
    pub last_result: Option<QueryResult>,
    pub result_row: usize,
    pub status: String,
    pub row_limit: u32,
}

pub struct ConnState {
    pub cfg: Connection,
    /// Lazily opened on first command. ConnectionManager auto-
    /// reconnects under the hood, so we hold it across many runs.
    pub client: Option<ConnectionManager>,
    pub last_error: Option<String>,
}

impl App {
    pub async fn new(cfg: Config) -> Result<Self> {
        let connections: Vec<ConnState> = cfg
            .connections
            .iter()
            .map(|c| ConnState {
                cfg: c.clone(),
                client: None,
                last_error: None,
            })
            .collect();
        let row_limit = cfg.row_limit;
        Ok(App {
            cfg,
            connections,
            active: 0,
            query: String::new(),
            cursor: 0,
            last_result: None,
            result_row: 0,
            status: "press Ctrl+Enter to run · Alt+1-9 switch connection · q quit".to_string(),
            row_limit,
        })
    }

    pub fn active_name(&self) -> &str {
        &self.connections[self.active].cfg.name
    }

    pub fn switch_connection(&mut self, idx: usize) {
        if idx < self.connections.len() {
            self.active = idx;
            self.status = format!("connection: {}", self.connections[idx].cfg.name);
        }
    }

    async fn ensure_connected(&mut self) -> Result<()> {
        let idx = self.active;
        if self.connections[idx].client.is_some() {
            return Ok(());
        }
        let url = self.connections[idx].cfg.url.clone();
        match connect(&url).await {
            Ok(c) => {
                self.connections[idx].client = Some(c);
                self.connections[idx].last_error = None;
                Ok(())
            }
            Err(e) => {
                self.connections[idx].last_error = Some(e.to_string());
                Err(e)
            }
        }
    }

    pub async fn run_query(&mut self) {
        if self.query.trim().is_empty() {
            self.status = "command is empty".to_string();
            return;
        }
        self.status = format!("running on {}…", self.active_name());
        if let Err(e) = self.ensure_connected().await {
            self.status = format!("connect failed: {e}");
            return;
        }
        let idx = self.active;
        let cmd = self.query.clone();
        let limit = self.row_limit;
        let conn = self.connections[idx].client.as_mut().unwrap();
        match run_command(conn, &cmd, limit).await {
            Ok(result) => {
                let n = result.rows.len();
                let total = result.server_row_count;
                let ms = result.elapsed.as_millis();
                self.status = if result.truncated {
                    format!("{n} of {total} rows · {ms}ms · truncated (press R to double limit)")
                } else {
                    format!("{n} rows · {ms}ms")
                };
                self.result_row = 0;
                self.last_result = Some(result);
            }
            Err(e) => {
                self.last_result = None;
                self.status = format!("error: {e}");
                self.connections[idx].last_error = Some(e.to_string());
            }
        }
    }

    pub fn move_result_row(&mut self, delta: isize) {
        let Some(result) = self.last_result.as_ref() else {
            return;
        };
        if result.rows.is_empty() {
            return;
        }
        let s = self.result_row as isize + delta;
        let new = s.clamp(0, result.rows.len() as isize - 1) as usize;
        self.result_row = new;
    }

    pub fn query_insert(&mut self, c: char) {
        let byte = self
            .query
            .char_indices()
            .nth(self.cursor)
            .map(|(b, _)| b)
            .unwrap_or_else(|| self.query.len());
        self.query.insert(byte, c);
        self.cursor += 1;
    }

    pub fn query_backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = self
            .query
            .char_indices()
            .nth(self.cursor - 1)
            .map(|(b, _)| b)
            .unwrap_or(0);
        let end = self
            .query
            .char_indices()
            .nth(self.cursor)
            .map(|(b, _)| b)
            .unwrap_or_else(|| self.query.len());
        self.query.replace_range(start..end, "");
        self.cursor -= 1;
    }

    pub fn query_clear(&mut self) {
        self.query.clear();
        self.cursor = 0;
    }

    pub fn double_row_limit(&mut self) {
        self.row_limit = self.row_limit.saturating_mul(2);
        self.status = format!("row_limit = {} — re-run with Ctrl+Enter", self.row_limit);
    }
}
