# Contributing to WaveMilestone

Thank you for your interest in contributing! Please follow these guidelines to keep the project maintainable and high-quality.

## Code of Conduct

By participating, you agree to uphold a respectful, inclusive environment. Harassment or discriminatory behavior will not be tolerated.

## Getting Started

1. Fork the repository.
2. Clone your fork: `git clone https://github.com/<your-username>/wave-milestone.git`
3. Run `./scripts/setup.sh` to install tooling and verify the build.
4. Create a feature branch: `git checkout -b feat/my-feature`

## Development Workflow

### Branch Naming

- `feat/` — new features
- `fix/` — bug fixes
- `chore/` — maintenance, dependencies
- `docs/` — documentation changes
- `test/` — test additions or improvements
- `perf/` — performance improvements
- `refactor/` — code restructuring

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Examples:
```
feat(contract): add milestone pool expiry validation
fix(contract): revert duplicate claim with correct error code
test(integration): add over-allocation grace period test
docs(readme): update deployment instructions
```

### Pre-commit Hooks

Install pre-commit hooks to auto-format and lint:

```bash
pre-commit install --hook-type pre-commit --hook-type pre-push
```

This runs:
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test` (on push only)

## Testing

### Running Tests

```bash
# All tests
./scripts/test.sh

# Specific test
./scripts/test.sh test_duplicate_claim_rejected

# Integration tests only
cargo test --workspace --test '*' -- --nocapture
```

### Writing Tests

- **Unit tests**: Add to `contracts/wave_milestone/src/test.rs` — test individual function behavior, error paths, edge cases.
- **Integration tests**: Add a new file in `contracts/wave_milestone/tests/` — test cross-contract interactions and full lifecycle scenarios.
- **Test helpers**: Shared setup code goes in `contracts/wave_milestone/tests/common/`.
- **Mock contracts**: Use `MockToken` and `MockWaveGuard` for deterministic testing without real on-chain dependencies.

### Test Coverage Expectations

All new features must include:
1. **Happy path** test confirming the feature works.
2. **Error path** test(s) confirming each error condition is handled.
3. **Edge case** tests for boundary conditions (zero amounts, expiration boundaries, duplicate calls).

## Code Style

- **Formatting**: 4-space indentation, 120-character line limit, Unix line endings. Run `cargo fmt` before committing.
- **Linting**: `cargo clippy` must pass with `-D warnings`. The `clippy.toml` enforces strict lint levels.
- **No panics**: Contract code must never panic. All error paths must return `Result<_, Error>`.
- **No unsafe**: The contract uses `#![no_std]` and prohibits `unsafe` code.
- **Imports**: Grouped and reordered per `rustfmt.toml` configuration.

### Contract-Specific Rules

1. **Authentication first**: Every public method must call `require_auth()` before any state mutation.
2. **Check-effects-interaction**: Validate inputs → Update state → Emit events → Transfer tokens.
3. **Balance checks before transfers**: Never attempt a token transfer without confirming sufficient balance.
4. **Duplicate protection**: All claim operations must check and set `completed` in the same atomic operation.

## Pull Request Process

1. Ensure all tests pass and CI is green.
2. Update documentation if adding or changing public interfaces.
3. Add or update tests to cover your changes.
4. Request review from a project maintainer.
5. Squash commits before merge (use `git rebase -i`).

### PR Checklist

- [ ] Code follows the project's Rust style
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] New tests added for all logic changes
- [ ] Documentation updated (README, ARCHITECTURE, etc.)
- [ ] No `dbg!()`, `todo!()`, `unreachable!()`, or `panic!()` in production code
- [ ] Commit messages follow Conventional Commits

## Release Process

Maintainers follow this process:

1. Ensure `main` is green on CI.
2. Update version in workspace `Cargo.toml`.
3. Tag the release: `git tag v<semver> && git push origin v<semver>`.
4. The [Release workflow](../.github/workflows/release.yml) builds, optimizes, and publishes the WASM artifact.
5. Publish release notes on GitHub.

## Security Disclosures

Report security vulnerabilities privately to the maintainers. Do not open public issues for security bugs. See [SECURITY.md](./SECURITY.md) for details.

## Questions?

Open a [Discussion](https://github.com/anomalyco/wave-milestone/discussions) or reach out to the maintainers.
