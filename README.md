# CodeVerse Compositor

A work-in-progress Wayland compositor written in Rust, built on top of [Smithay](https://github.com/Smithay/smithay).

This repository is a Rust workspace containing the compositor plus supporting crates (configuration, IPC, window/layout logic, and launcher).

## Workspace crates

- `crates/codeverse-compositor`: The compositor binary (`codeverse-compositor`)
- `crates/codeverse-window`: Window management, layouts, and workspace tree
- `crates/codeverse-config`: TOML configuration + keybindings + theme
- `crates/codeverse-ipc`: IPC types/helpers
- `crates/codeverse-launcher`: App discovery / `.desktop` parsing

## Requirements

- Linux
- Rust toolchain (edition 2021)
- System libraries required by Smithay/DRM/input stacks (exact package names vary by distro). Commonly needed:
  - `libxkbcommon`
  - `libinput`
  - `libseat`
  - `libdrm` + GBM/Mesa/EGL
  - `pkg-config` (often required to locate system libs)

If you hit build errors about missing `-dev`/`-devel` packages, install the corresponding development headers for your distro.

## Build

- Debug build:
  - `cargo build`
- Release build:
  - `cargo build --release`

To build just the compositor crate:
- `cargo build -p codeverse-compositor`

## Run

The binary auto-selects a backend:
- **Winit backend (nested)** when `DISPLAY` or `WAYLAND_DISPLAY` is set (i.e. launched inside an existing X11/Wayland session)
- **DRM backend (TTY / “real” session)** otherwise

Run from an existing desktop session (nested):
- `RUST_LOG=info cargo run -p codeverse-compositor`

Run on a clean TTY (DRM):
- `RUST_LOG=info cargo run -p codeverse-compositor --release`

Notes:
- DRM/TTY mode typically requires correct device permissions (e.g. `video`/`input` groups) and a seat/session setup.
- If you’re unsure, start with nested mode first.

## Configuration

- Default config path:
  - `~/.config/codeverse-compositor/config.toml`
- Example config shipped in this repo:
  - `config/default.toml`

To start customizing:
- `mkdir -p ~/.config/codeverse-compositor`
- `cp config/default.toml ~/.config/codeverse-compositor/config.toml`

## Testing in a VM (QEMU)

There is a helper script that boots a Linux live ISO and copies the compositor binary into a shared folder:
- `./scripts/qemu-test.sh`

There is also a convenience deploy script for a running VM:
- `./scripts/deploy-to-vm.sh`

## Development

Useful commands:
- Format: `cargo fmt --all`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Tests: `cargo test --workspace`

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Security

See [SECURITY.md](SECURITY.md) for reporting vulnerabilities.

## License

MIT licensed. See [LICENSE](LICENSE).
