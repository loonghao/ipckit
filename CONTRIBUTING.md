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

## Release Process

### Setting Up Secrets

To enable automatic publishing, you need to configure the following secrets in your GitHub repository:

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

### Creating a Release

1. Update version in:
   - `Cargo.toml` (workspace version)
   - `pyproject.toml`
   - `crates/ipckit/Cargo.toml`

2. Commit and push:
   ```bash
   git add .
   git commit -m "Bump version to x.y.z"
   git push
   ```

3. Create and push a tag:
   ```bash
   git tag vx.y.z
   git push origin vx.y.z
   ```

4. The release workflow will automatically:
   - Build wheels for all platforms
   - Publish to PyPI
   - Publish to crates.io
   - Create a GitHub release

## Code Style

### Rust

- Follow standard Rust formatting (`cargo fmt`)
- No clippy warnings (`cargo clippy`)

### Python

- Follow PEP 8
- Use type hints where possible

## Pull Request Process

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests
5. Submit a pull request

## License

By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.
