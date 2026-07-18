# hnw

`hnw` is a keyboard-first terminal manager for [here.now](https://here.now). It reads the same API key as the official scripts and never displays or stores a second copy of it.

## Install

```bash
cargo install --path .
hnw
```

Authentication is resolved in this order:

1. `HERENOW_API_KEY`
2. `~/.herenow/credentials`
3. A file passed with `hnw --credentials <path>`

## What it manages

- Sites: list, search indexed content, inspect file manifests, rename, duplicate, open, view 30-day analytics, and batch-delete selected Sites with confirmation.
- Drives: list Drives, browse their files, and open the selected Drive in the here.now dashboard.
- Account: inspect and open the public profile.

Press `?` in the app for the complete keyboard map. The common keys are `Tab` to change section, `j`/`k` to move, `Space` to mark Sites, `A` to mark all visible Sites, `Esc` to clear marks, `x` to delete the marked set, `r` to refresh, and `q` to quit.

## Safety

- Delete is the only hard-destructive action in this release. The confirmation lists the selected Sites and requires an explicit `y`.
- Secrets are passed only through the HTTP `Authorization` header and are never rendered or logged.
- The default API origin is `https://here.now`; `--base-url` exists for testing.

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
