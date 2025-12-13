# Contributing to ipckit

Thank you for your interest in contributing to ipckit!

## Development Setup

### Prerequisites

- Rust 1.70+
- Python 3.8+
- maturin (`pip install maturin`)

### Building

```bash
# Clone the repository
git clone https://github.com/loonghao/ipckit.git
cd ipckit

# Build in development mode
maturin develop

# Build in release mode
maturin develop --release

# Run Rust tests
cargo test

# Run Python tests
pip install pytest pytest-timeout
pytest tests/
```

## Commit Convention

This project uses [Conventional Commits](https://www.conventionalcommits.org/) for automatic changelog generation.

### Commit Types

| Type | Description |
|------|-------------|
| `feat` | A new feature |
| `fix` | A bug fix |
| `docs` | Documentation only changes |
| `style` | Code style changes (formatting, etc.) |
| `refactor` | Code refactoring |
| `perf` | Performance improvements |
| `test` | Adding or updating tests |
| `chore` | Maintenance tasks |
| `ci` | CI/CD changes |
| `build` | Build system changes |

### Examples

```bash
feat: add WebSocket-based IPC channel
fix: resolve race condition in SharedMemory
docs: update FileChannel usage examples
perf: optimize JSON serialization by 30%
```

## Release Process

This project uses [Release Please](https://github.com/googleapis/release-please) for automated releases.

### How It Works

1. **Commit with Conventional Commits** - Use proper commit prefixes (`feat:`, `fix:`, etc.)
2. **Release Please creates a PR** - When commits are pushed to `main`, Release Please automatically creates/updates a release PR
3. **Merge the release PR** - When ready to release, merge the release PR
4. **Automatic publishing** - The release workflow will:
   - Build wheels for all platforms (Windows, Linux, macOS)
   - Publish to PyPI
   - Publish to crates.io
   - Create a GitHub Release with artifacts

### Setting Up Secrets

To enable automatic publishing, configure the following secrets in your GitHub repository:

#### 1. PyPI Publishing (Trusted Publisher - Recommended)

1. Go to https://pypi.org/manage/account/publishing/
2. Add a new pending publisher:
   - PyPI Project Name: `ipckit`
   - Owner: `loonghao`
   - Repository name: `ipckit`
   - Workflow name: `release.yml`
   - Environment name: `pypi`

3. In GitHub repository settings:
   - Go to Settings → Environments
   - Create a new environment named `pypi`
   - (Optional) Add protection rules

#### 2. crates.io Publishing

1. Go to https://crates.io/settings/tokens
2. Create a new token with `publish-update` scope
3. In GitHub repository settings:
   - Go to Settings → Secrets and variables → Actions
   - Add a new repository secret:
     - Name: `CARGO_REGISTRY_TOKEN`
     - Value: Your crates.io API token

### Manual Release (if needed)

If you need to trigger a release manually:

1. Go to Actions → Release → Run workflow
2. Optionally check "Force rebuild wheels"

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test && pytest tests/`)
5. Commit with Conventional Commits format
6. Push to your fork
7. Open a Pull Request

### PR Checklist

- [ ] PR title follows Conventional Commits
- [ ] Tests added/updated
- [ ] Documentation updated if needed
- [ ] CI passes

## Code Style

### Rust

- Follow standard Rust formatting (`cargo fmt`)
- No clippy warnings (`cargo clippy -- -D warnings`)

### Python

- Follow PEP 8
- Use type hints where possible

## License

By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.
