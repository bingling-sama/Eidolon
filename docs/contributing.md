# Contributing

Contributions are welcome.

## How to Contribute

- Open an issue to discuss bugs, ideas, or proposals
- Submit a pull request with a clear description of the change
- Keep changes focused and include usage notes or examples when relevant

## Development Notes

- Build with `cargo build`
- Format with `cargo fmt --check`
- Lint with `cargo clippy --all-targets --all-features`
- Test with `cargo test`
- Run the CLI examples from `docs/cli.md`
- Run `cargo bench` when changing render performance-sensitive paths

## Documentation Changes

When changing CLI options, public library fields, resource names, or output behavior, update the
matching English and Chinese documentation files. The documentation index lives in `docs/README.md`
and `docs/README_zh-CN.md`.

## Pull Request Checklist

- Keep the change scoped to one behavior or documentation topic
- Include screenshots or rendered output when visual behavior changes
- Mention any platform-specific GPU or windowing requirements
- Note any known limitations that remain after the change
