//! `redis` crate wrapper. v0.1 parses a command line into argv,
//! sends it as a `redis::cmd`, and renders the response value
//! using a small recursive formatter that flattens nested arrays
//! into a `Vec<Vec<String>>` for the TUI table.

use anyhow::{Context, Result};
use redis::{Value, aio::ConnectionManager};

/// Open a Redis connection from a redis:// URL.
pub async fn connect(url: &str) -> Result<ConnectionManager> {
    let client = redis::Client::open(url).context("parsing Redis URL")?;
    let manager = ConnectionManager::new(client)
        .await
        .context("connecting to Redis")?;
    Ok(manager)
}

/// Returned cells + columns, ready for the TUI's table widget.
#[derive(Debug, Clone, Default)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub elapsed: std::time::Duration,
    pub server_row_count: usize,
    pub truncated: bool,
}

/// Run a Redis command line. The whole line is whitespace-split
/// into `(verb, ...args)` and sent as a single `cmd`. Quoted args
/// aren't supported in v0.1 — most real commands don't need them
/// (`SCAN 0 MATCH user:* COUNT 100`, `HGETALL my-key`, etc.).
pub async fn run_command(
    conn: &mut ConnectionManager,
    line: &str,
    row_limit: u32,
) -> Result<QueryResult> {
    let mut parts = line.split_whitespace();
    let Some(verb) = parts.next() else {
        anyhow::bail!("empty command");
    };
    let args: Vec<&str> = parts.collect();
    let mut cmd = redis::cmd(verb);
    for a in &args {
        cmd.arg(*a);
    }
    let start = std::time::Instant::now();
    let value: Value = cmd
        .query_async(conn)
        .await
        .with_context(|| format!("running `{line}`"))?;
    let elapsed = start.elapsed();

    let (columns, all_rows) = format_value(&value);
    let server_row_count = all_rows.len();
    let take = (row_limit as usize).min(all_rows.len());
    let truncated = all_rows.len() > take;
    let rows = all_rows.into_iter().take(take).collect();
    Ok(QueryResult {
        columns,
        rows,
        elapsed,
        server_row_count,
        truncated,
    })
}

/// Map a Redis response value to (column headers, rows). Handles
/// the common shapes:
///   - Scalar (Int / BulkString / SimpleString / Nil) → 1 row × `[value]`
///   - Array of scalars (`KEYS`, `SMEMBERS`) → N rows × `[member]`
///   - Array of pairs (`HGETALL`, `ZRANGE WITHSCORES`) → N rows × `[key, value]`
///   - Nested array — flattened (one row per inner element).
fn format_value(v: &Value) -> (Vec<String>, Vec<Vec<String>>) {
    match v {
        Value::Nil => (vec!["value".to_string()], vec![vec!["nil".to_string()]]),
        Value::Int(n) => (vec!["value".to_string()], vec![vec![n.to_string()]]),
        Value::BulkString(bytes) => {
            let s = String::from_utf8_lossy(bytes).to_string();
            (vec!["value".to_string()], vec![vec![s]])
        }
        Value::SimpleString(s) => (vec!["value".to_string()], vec![vec![s.clone()]]),
        Value::Okay => (vec!["value".to_string()], vec![vec!["OK".to_string()]]),
        Value::Array(items) | Value::Set(items) => {
            // Heuristic: if it's a flat array of scalars with an even
            // count, treat as key/value pairs. Otherwise one-cell-per-row.
            let all_scalar = items.iter().all(|v| {
                matches!(
                    v,
                    Value::Nil
                        | Value::Int(_)
                        | Value::BulkString(_)
                        | Value::SimpleString(_)
                        | Value::Okay
                )
            });
            if all_scalar && items.len() % 2 == 0 && !items.is_empty() {
                let rows: Vec<Vec<String>> = items
                    .chunks(2)
                    .map(|pair| vec![scalar_to_string(&pair[0]), scalar_to_string(&pair[1])])
                    .collect();
                (vec!["field".to_string(), "value".to_string()], rows)
            } else if all_scalar {
                let rows: Vec<Vec<String>> =
                    items.iter().map(|v| vec![scalar_to_string(v)]).collect();
                (vec!["value".to_string()], rows)
            } else {
                // Mixed/nested — fall back to recursive flatten.
                let rows: Vec<Vec<String>> = items
                    .iter()
                    .flat_map(|v| {
                        let (_, sub) = format_value(v);
                        sub
                    })
                    .collect();
                (vec!["value".to_string()], rows)
            }
        }
        Value::Map(pairs) => {
            let rows: Vec<Vec<String>> = pairs
                .iter()
                .map(|(k, v)| vec![scalar_to_string(k), scalar_to_string(v)])
                .collect();
            (vec!["field".to_string(), "value".to_string()], rows)
        }
        _ => (vec!["value".to_string()], vec![vec![format!("{v:?}")]]),
    }
}

fn scalar_to_string(v: &Value) -> String {
    match v {
        Value::Nil => "nil".to_string(),
        Value::Int(n) => n.to_string(),
        Value::BulkString(b) => String::from_utf8_lossy(b).to_string(),
        Value::SimpleString(s) => s.clone(),
        Value::Okay => "OK".to_string(),
        _ => format!("{v:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_int_renders_one_row_one_col() {
        let (cols, rows) = format_value(&Value::Int(42));
        assert_eq!(cols, vec!["value"]);
        assert_eq!(rows, vec![vec!["42".to_string()]]);
    }

    #[test]
    fn bulk_string_renders_as_utf8() {
        let (cols, rows) = format_value(&Value::BulkString(b"hello".to_vec()));
        assert_eq!(cols, vec!["value"]);
        assert_eq!(rows, vec![vec!["hello".to_string()]]);
    }

    #[test]
    fn keys_style_array_renders_one_row_per_member() {
        // KEYS user:* response shape.
        let v = Value::Array(vec![
            Value::BulkString(b"user:1".to_vec()),
            Value::BulkString(b"user:2".to_vec()),
            Value::BulkString(b"user:3".to_vec()),
        ]);
        let (cols, rows) = format_value(&v);
        // 3 elements, odd-count → one row per member.
        assert_eq!(cols, vec!["value"]);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0][0], "user:1");
    }

    #[test]
    fn hgetall_style_array_renders_as_pairs() {
        // HGETALL returns flat [field, value, field, value, ...].
        let v = Value::Array(vec![
            Value::BulkString(b"name".to_vec()),
            Value::BulkString(b"alice".to_vec()),
            Value::BulkString(b"age".to_vec()),
            Value::BulkString(b"30".to_vec()),
        ]);
        let (cols, rows) = format_value(&v);
        assert_eq!(cols, vec!["field", "value"]);
        assert_eq!(
            rows,
            vec![
                vec!["name".to_string(), "alice".to_string()],
                vec!["age".to_string(), "30".to_string()],
            ]
        );
    }

    #[test]
    fn nil_renders_as_literal_nil() {
        let (_, rows) = format_value(&Value::Nil);
        assert_eq!(rows[0][0], "nil");
    }
}
