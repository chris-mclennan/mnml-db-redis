# mnml-db-redis

Redis command playground for [mnml](https://mnml.sh) — terminal TUI
with multiple saved connections. Same shape as the SQL-flavored
siblings (`mnml-db-postgres` / `-mariadb` / `-redshift` /
`-clickhouse`), but the "query" is a Redis command line and the
response is rendered via small type-aware formatting.

```
┌─ connections ────────────────────────────────────────────────────┐
│ ▸● Alt+1 local   ○ Alt+2 prod-cache                              │
└──────────────────────────────────────────────────────────────────┘
┌─ command @ local ────────────────────────────────────────────────┐
│                                                                  │
│ HGETALL user:1234│                                               │
│                                                                  │
│   Ctrl+Enter / F5 run · Ctrl+U clear · Ctrl+↑/↓ scroll results   │
└──────────────────────────────────────────────────────────────────┘
┌─ results (3 · 2ms) ──────────────────────────────────────────────┐
│ field    │ value                                                  │
│ name     │ alice                                                  │
│ email    │ alice@example.com                                      │
│ age      │ 30                                                     │
└──────────────────────────────────────────────────────────────────┘
```

## Install

```sh
cargo install --git https://github.com/chris-mclennan/mnml-db-redis mnml-db-redis
```

## Setup

1. **Run once** to scaffold config:
   ```sh
   mnml-db-redis
   ```
   Writes `~/.config/mnml-db-redis.toml`. `chmod 600` it.

2. **Edit `[[connections]]`**:
   ```toml
   [[connections]]
   name = "local"
   url  = "redis://localhost:6379"

   [[connections]]
   name = "prod-cache"
   url  = "rediss://:${PROD_REDIS_PASS}@redis.prod.example.com:6380/0"
   ```

3. **Re-run** — TUI launches; type a Redis command, `Ctrl+Enter` to run.

## Response formatting

Responses are rendered via small type-aware formatting:

| Redis response shape           | Table view                  |
|--------------------------------|-----------------------------|
| Scalar (string, int, nil)      | 1 row × `[value]`            |
| Array of strings (KEYS, SMEMBERS) | N rows × `[member]`       |
| Pair-array (HGETALL, ZRANGE WITHSCORES) | N rows × `[field, value]` |
| Map response                   | N rows × `[field, value]`    |
| Error                          | Surfaces in the status line  |

NULLs render as the literal `nil`. Binary-safe strings (bytes) are
UTF-8 lossily decoded; binary blobs may appear as � characters but
won't crash the renderer.

## Keys

Same family chord set:

| Chord                | Action                                            |
|----------------------|---------------------------------------------------|
| `Ctrl+Enter` / `F5`  | Run the current command                           |
| `Alt+1`-`Alt+9`      | Switch to that connection                         |
| `Ctrl+U`             | Clear the command buffer                          |
| `Ctrl+↑/↓` / `Ctrl+P/N` | Move selection in the results table             |
| `R` (uppercase)      | Double `row_limit` for the next run               |
| `q` / `Esc` / `Ctrl+C` | Quit                                            |

## Limitations (v0.1)

- **No quoted argument parsing.** Commands are whitespace-split. Most
  real commands don't need quoting (`SCAN 0 MATCH user:* COUNT 100`,
  `HGETALL my-key`), but a value with embedded spaces won't round-
  trip. Use `redis-cli` or a write-friendly client for those.
- **Single-statement only.** Pipelines / transactions are v0.2.
- **Read-only by convention.** Nothing prevents `FLUSHDB` — use a
  Redis user with `+@read` / `-@dangerous` ACL for prod URLs.

## License

MIT.
