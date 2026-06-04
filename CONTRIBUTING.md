# Contributing to mnml-db-redis

Thanks for taking a look! This repo is part of the [mnml integration family](https://mnml.sh/manual/integrations/community/) — a standalone Redis command playground that doubles as a hosted mnml pane.

## Two paths

**A. You want to fix a bug or add a Redis-specific feature here.** Open an issue or PR against this repo. See "Local development" below.

**B. You want a viewer for a different backend.** **Fork this repo** and replace `src/redis_client.rs` with your backend. The rest of the scaffold (`blit.rs`, `config.rs`, `ui.rs`, `keys.rs`, `app.rs`) is designed to be copy-pasted. See [Building integrations](https://mnml.sh/manual/integrations/building/) for the full guide. You don't owe anything back to this repo or to mnml — your fork can live under your own name.

## Project layout

```
src/
├── main.rs                # CLI + mode dispatch (TUI / --blit / --check)
├── app.rs                 # state — connections, command buffer, results
├── config.rs              # ~/.config/mnml-db-redis.toml
├── redis_client.rs        # ← the backend-specific file (swap this when forking)
├── keys.rs                # action enum + key bindings
├── ui.rs                  # ratatui draw + crossterm loop
└── blit.rs                # tmnl-protocol over UDS — copied verbatim
```

This one is a good fork target for **NoSQL / KV / command-style backends** (Memcached, etcd, an internal RPC service) — the input is a single command line and the response is rendered with small type-aware formatting (scalar / array / pair-array). The SQL-flavored siblings assume rows-and-columns, which doesn't fit those shapes.

`blit.rs` is shared verbatim across the family. Patches to `blit.rs` should land first in [`mnml-db-postgres`](https://github.com/chris-mclennan/mnml-db-postgres) and then be ported to the siblings.

## Local development

```sh
git clone https://github.com/chris-mclennan/mnml-db-redis
cd mnml-db-redis
cargo build
cargo test
cargo clippy --all-targets        # must be warning-free
cargo fmt                          # before committing
```

Spin up a local Redis for manual testing:

```sh
docker run -d --name redis-mnml -p 6379:6379 redis:7
cargo run -- --check
cargo run
```

## PR conventions

- One commit per logical change is fine; squash on merge is fine too.
- Commit messages: short imperative subject (≤72 chars), optional body explaining "why".
- Add a unit test for any backend behavior you change.
- `cargo clippy --all-targets` and `cargo fmt --check` must be clean.

## License + ownership

MIT. Contributions are accepted under the same license. No copyright assignment required; you keep authorship of your changes.
