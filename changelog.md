# Version 2.0.1
* fixed alias chain resolving for `ran alias list` and `ran alias get <alias>`.
* fixed a bug in which `ran config set alias.KEY` would set to `config.env` map instead of `config.alias` map.

# Version 2.0.0
* complete overhaul of ran's internal architecture; ~60% rewrite of the codebase.
* centralized CLI handling in `handler.rs`, drastically reducing `main.rs` size and spaghetti code.
* added `resolver.rs` to handle alias chains, variable expansion, and per-app command resolution.
* switched all error handling to `anyhow`; removed old custom error types.
* reorganized utilities into `util` module submodules (`args`, `fs`, `table`, etc.) instead of a single `utils.rs`.
* editor field removed; now editing config or app definition files uses `$EDITOR`/`$VISUAL`, OS-wide default app for `.toml` files, or falls back to `nano`/`notepad`.
* interactive mode inverted to `noninteractive` (false by default); interactive prompts respect this flag.
* added per-app variables, env overrides per command, and preserved app env settings while running commands.
* commands can now be defined per app; launching supports both `launch` (default command) and named commands (`cmd`).
* improved CLI: new commands, subcommands, and flags for apps, config, alias, and variable management.
* colored alias chains and better error formatting for clarity.
* most internal maps now use `IndexMap` to preserve insertion order (minor reordering occurs on removal via `swap_remove`).
* saving/loading apps and config now supports nested maps, arrays, and clean serialization using `toml_edit`.
* gracefully handles missing `config_path/apps/` directory without errors or hiccups.
* numerous small bug fixes and code cleanups throughout (`resolver`, `launcher`, `cli`, `app`, `config`).

check [readme.md](https://github.com/hasibix/ran/blob/main/readme.md) on how to update your app definitions and config files.

# Version 1.0.2
* fixed config path resolution to make it work correctly across platforms.
* fixed issues on Windows where `ran` would refuse to run when unable to resolve a default config path, even if a config path was provided via $RANCFG.

# Version 1.0.1
* made launched programs run in the same directory as `ran` was executed from.

# Version 1.0.0
* initial release
