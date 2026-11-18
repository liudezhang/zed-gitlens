# GitLens Blame for Zed

A Zed extension that displays Git blame details at the end of each source line using LSP inlay hints.

## Features

- Per-line author, relative commit time, and commit summary.
- Uses the current Git worktree and respects the installed `git` executable.
- Runs alongside the language server already configured for each language.

## Requirements

- Windows, Rust, Git, and Zed with extension API 0.7.0 support.
- The `wasm32-wasip2` Rust target installed.
- Git available in PATH.
- Zed inlay hints enabled (`"inlay_hints": { "enabled": true }`) when using the experimental LSP.

## Development

The extension consists of a Zed WASM extension and a native Rust LSP server.

```sh
cargo build --release -p gitlens-blame
cargo build --release --target wasm32-wasip2 -p zed-gitlens-extension
```

## Local installation

This is a local-use prototype. It does not download binaries or require GitHub Releases.
The server path is currently hardcoded in `extension/src/lib.rs` to:

```text
C:\Users\Administrator\AppData\Local\Zed\extensions\work\gitlens\gitlens-blame.exe
```

On another machine, change that path before compiling. Copy `target/release/gitlens-blame.exe` to the configured location.

For the manual installation used on this machine, copy `extension/extension.toml` and `target/wasm32-wasip2/release/zed_gitlens_extension.wasm` into `%LOCALAPPDATA%/Zed/extensions/installed/gitlens/`, renaming the WASM file to `extension.wasm`. Restart Zed after replacing the files; close the running server before replacing its executable.

## Current-line blame and Git Graph

The experimental LSP displays per-line hints, not just the active line. The final local setup uses Zed's native inline blame and Git Graph instead, with the custom LSP disabled. These user settings are not installed by this extension.

To use the native current-line display without enabling this experimental LSP, add the following to your Zed user settings (merge with existing settings):

```json
{
  "git": {
    "inline_blame": {
      "enabled": true,
      "location": "inline",
      "delay_ms": 300,
      "show_commit_summary": true
    }
  },
  "language_servers": ["!gitlens-blame", "..."]
}
```

On Zed versions that include Git Graph, run `git graph: open` in the command palette. The local shortcut is defined in the user keymap, not in this extension:

```json
[
  {
    "context": "Workspace",
    "bindings": {
      "ctrl-alt-g": "git_graph::Open"
    }
  }
]
```

Git Graph opens in an editor tab. This project does not provide a custom graphical bottom panel.
