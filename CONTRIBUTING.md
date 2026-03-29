# Contributing

Thanks for contributing to CodeVerse Compositor!

## Quick start

1. Fork the repo and create a feature branch.
2. Make your change.
3. Run the checks locally:
   - `cargo fmt --all`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
4. Open a pull request.

## Development tips

### Logs

Enable logs with `RUST_LOG`:
- `RUST_LOG=info cargo run -p codeverse-compositor`
- `RUST_LOG=debug cargo run -p codeverse-compositor`

If you’re debugging a crash, also try:
- `RUST_BACKTRACE=1`

### Backends

The compositor auto-selects the backend:
- Winit backend when `DISPLAY` or `WAYLAND_DISPLAY` is set (nested)
- DRM backend otherwise (TTY)

When working on DRM/TTY issues, it’s often easiest to reproduce in a VM first:
- `./scripts/qemu-test.sh`

## Filing issues

Good issues help us fix things quickly. Please include:

- What you expected vs what happened
- Your distro + kernel version
- Whether you ran nested (winit) or TTY (DRM)
- Logs (`RUST_LOG=debug` output) and backtrace (if there was a panic)
- Steps to reproduce (ideally minimal)

If the issue is a **security vulnerability**, do not open a public issue; use the process in `SECURITY.md`.

## Pull requests

- Keep PRs focused (one change/theme per PR when practical).
- Prefer small, reviewable commits.
- If you’re changing behavior, include a short explanation in the PR description.
- Add tests when there’s a natural place for them (this repo already contains unit tests in some crates).

## Code style

- Rust formatting is enforced via `rustfmt`.
- Prefer clear names over cleverness.
- Avoid unrelated refactors in the same PR.

## License

By contributing, you agree that your contributions will be licensed under the MIT License (see `LICENSE`).
