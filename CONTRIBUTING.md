# Contributing

## Getting Started

1. Fork the repo
2. Clone your fork
3. Create a branch: `git checkout -b feat/my-feature`
4. Make your changes
5. Build: `cargo build`
6. Test manually: `cargo run -- -u <UNIVERSE_ID> -c <CLIENT_ID>`
7. Commit (see below)
8. Push and open a PR

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>: <description>
```

Types:
- `feat` — new feature
- `fix` — bug fix
- `docs` — documentation only
- `ci` — CI/workflow changes
- `refactor` — code change that neither fixes a bug nor adds a feature
- `chore` — dependencies, config, misc

Examples:
```
feat: add --no-timestamps flag
fix: handle missing thumbnail gracefully
docs: update CLI usage in README
ci: bump actions/checkout to v7
```

## PR Guidelines

- Keep PRs focused — one change per PR
- Make sure `cargo build` succeeds
- Update README if adding/changing features
- Update CHANGELOG.md under an `[Unreleased]` section

## Development

```bash
# Debug build
cargo build

# Run with verbose logging
RUST_LOG=debug cargo run -- -u <UNIVERSE_ID> -c <CLIENT_ID>
```

## Release Process

Maintainers only:

1. Update `CHANGELOG.md` — add version header and date
2. Commit: `git commit -m "chore: release v1.2.0"`
3. Tag: `git tag -a v1.2.0 -m "v1.2.0"`
4. Push: `git push origin main --tags`
5. GitHub Actions builds binaries and creates the release
