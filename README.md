# Top Spin

A very simple GUI application (linux only), written in rust (with [druid](https://crates.io/crates/druid)). It stores a list of CLI program commands and run them in background with a click. Currently it throws away spawned processes' stdout/stderr, the plan is to add a view to stream logs.

![](./screenshots/v0.png)

## Config

A config file is read from `~/.config/topspin.toml` or file specified by `TOPSPIN_CONFIG` environment variable. Example:

```toml
[commands.cat]
command = "cat"

[commands.netcat]
command = "nc"
args ="-l 7000"
working_dir = "~/"
```