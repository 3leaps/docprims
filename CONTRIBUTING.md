# Contributing to docprims

Thank you for your interest in contributing to docprims.

## Code of Conduct

This project follows the [3leaps Code of Conduct](https://github.com/3leaps/oss-policies/blob/main/CODE_OF_CONDUCT.md).

## Getting Started

1. Fork the repository
2. Clone your fork
3. Run `make bootstrap` to install development tools
4. Create a feature branch
5. Make your changes
6. Run `make check` to verify quality gates pass
7. Submit a pull request

## Development Setup

### Prerequisites

- Rust 1.81+ (install via [rustup](https://rustup.rs/))
- curl (for bootstrap)

### Bootstrap

```bash
make bootstrap
```

This installs:

- sfetch (trust anchor binary fetcher)
- goneat (DX tool for code quality)
- cargo-deny (license and security checks)
- cargo-audit (security vulnerability scanner)

### Quality Gates

Before submitting changes:

```bash
make check      # Run all quality checks
make fmt        # Format code
make lint       # Run clippy
make test       # Run tests
make deny       # Check licenses
```

## Pull Request Process

1. Ensure all quality gates pass (`make check`)
2. Update documentation if needed
3. Add tests for new functionality
4. Follow existing code patterns
5. Use conventional commit messages

### Commit Messages

Follow the [3leaps commit style](https://crucible.3leaps.dev/repository/commit-style):

```
<type>(<scope>): <subject>

<body>

Co-Authored-By: <Model> <noreply@anthropic.com>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

## License

By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.

## Dependency Policy

docprims is **GPL-free by design**. All dependencies must be:

- MIT, Apache-2.0, BSD, ISC, or similarly permissive
- No GPL, LGPL, AGPL, or other copyleft licenses
- Run `cargo deny check licenses` to verify

## Security

If you discover a security vulnerability, please email security@3leaps.net rather than opening a public issue.
