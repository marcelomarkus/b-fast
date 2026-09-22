# Contributing to B-FAST

Thank you for your interest in contributing to B-FAST! We welcome contributions from the community and are excited to see what you'll bring to the project.

## 🌟 Philosophy

> "Knowledge is the only wealth that grows when we share it"

B-FAST is built on the principle of open knowledge sharing. Every contribution, no matter how small, helps make high-performance serialization accessible to more developers.

## 🚀 Getting Started

### Prerequisites
- **Rust** 1.70+ (for core library)
- **Python** 3.8+ with `uv` or `pip`
- **Node.js** 18+ (for TypeScript client)
- **Git** for version control

### Development Setup

1. **Fork and clone the repository:**
   ```bash
   git clone https://github.com/marcelomarkus/b-fast.git
   cd b-fast
   ```

2. **Set up Python environment:**
   ```bash
   uv sync --extra dev
   # or
   pip install bfast-py[dev]
   ```

3. **Install TypeScript dependencies:**
   ```bash
   cd client-ts
   npm install
   cd ..
   ```

4. **Run tests to verify setup:**
   ```bash
   # Python tests
   python -m pytest tests/
   
   # TypeScript build and tests
   cd client-ts && npm run build
   cd ..
   ```

## 🛠️ Development Workflow

### Making Changes

1. **Create a feature branch:**
   ```bash
   git checkout -b feat/your-feature-name
   # or
   git checkout -b fix/bug-description
   ```
   
   **Branch naming conventions:**
   - `feat/*` - New features or improvements (e.g., `feat/streamable-http`)
   - `fix/*` - Bug fixes (e.g., `fix/memory-leak`)
   - `docs/*` - Documentation only (e.g., `docs/contributing`)
   
   **Note:** CI automatically runs on `feat/*` and `fix/*` branches. Documentation-only changes (`docs/*`) don't trigger heavy test suites.

2. **Make your changes** following our coding standards.

3. **Run the full test suite:**
   ```bash
   # Python tests
   python -m pytest tests/ -v
   
   # Code formatting and linting
   python -m ruff check .
   
   # TypeScript build
   cd client-ts && npm run build && cd ..
   ```

4. **Commit with descriptive messages:**
   ```bash
   git commit -m "feat: add support for UUID serialization
   
   - Implement UUID type in Rust encoder
   - Add UUID parsing in TypeScript decoder
   - Include tests for UUID round-trip serialization"
   ```

## 📝 Contribution Types

### 🐛 Bug Reports
- Use the bug report template on GitHub
- Include a minimal reproduction case
- Specify Python, Node.js, and Rust versions
- Include error messages and stack traces

### ✨ Feature Requests
- Open a discussion or issue outlining the feature
- Explain the use case and real-world benefits
- Consider backward compatibility and protocol specifications

### 🔧 Code Contributions
- **Core Rust library:** `src/lib.rs`, `src/errors.rs`
- **Python bindings:** `python/b_fast/`
- **TypeScript client:** `client-ts/`
- **Documentation:** `docs/`, `zensical.toml`, `README.md`
- **Tests:** `tests/`

### 📚 Documentation
- API documentation improvements
- Usage examples and tutorials
- Performance optimization guides
- Translation to other languages

## 🎯 Coding Standards

### Python
- **Formatting & Linting:** Ruff with project configuration
- **Type hints:** Required for public APIs
- **Tests:** `pytest` with descriptive names

### Rust
- **Formatting:** `cargo fmt`
- **Linting:** `cargo clippy`
- **Documentation:** Rustdoc comments for public APIs
- **Safety:** Minimize `unsafe` code, document whenever necessary

### TypeScript
- **Formatting:** Prettier (via npm scripts)
- **Linting:** ESLint configuration
- **Types:** Strict TypeScript, avoid `any` in public APIs
- **Compatibility:** ES2020+ for modern environments

### 🏷️ Versioning (Single Source of Truth)
- The project version is defined strictly in `Cargo.toml` (`[package].version`).
- `pyproject.toml` uses `dynamic = ["version"]` and derives the Python wheel version automatically via Maturin.
- The Python runtime version `b_fast.__version__` is exposed directly by the compiled Rust extension via `env!("CARGO_PKG_VERSION")`.
- To sync the version to `client-ts/package.json`, run:
  ```bash
  cd client-ts && npm run sync-version
  ```

## 🧪 Testing Guidelines

### Test Categories
- **Unit tests:** Individual function and method testing
- **Integration tests:** Component interaction and serialization testing
- **Performance tests:** Benchmark critical paths ([Performance Guide](performance.md))
- **Compatibility tests:** Cross-platform and cross-language validation (Python ↔ TypeScript)

### Test Requirements
- All new features must include tests
- Bug fixes must include regression tests
- Performance changes should include benchmark comparisons
- Breaking changes require migration notes

## 📋 Pull Request Process

1. **Ensure your PR:**
   - Has a clear, descriptive title
   - References related issues (`Fixes #123`)
   - Includes tests for new functionality
   - Updates documentation if needed
   - Passes all CI checks

2. **PR Review Process:**
   - Maintainers will review within 48 hours
   - Address feedback promptly and constructively
   - Keep discussions respectful

3. **Merge Requirements:**
   - All tests passing
   - Code review approval
   - No merge conflicts
   - Documentation updated

## 🤝 Community Guidelines

- **Be Respectful:** Use inclusive language and respect different perspectives.
- **Be Collaborative:** Share knowledge, credit others' work, and help newcomers feel welcome.
- **Be Professional:** Keep discussions constructive and follow the Code of Conduct.

## 🆘 Getting Help

- **Discussions:** [GitHub Discussions](https://github.com/marcelomarkus/b-fast/discussions)
- **Issues:** [GitHub Issues](https://github.com/marcelomarkus/b-fast/issues)
- **Documentation:** [https://marcelomarkus.github.io/b-fast/](https://marcelomarkus.github.io/b-fast/)

---

Thank you for contributing to B-FAST! Together, we're making high-performance serialization accessible to everyone. 🚀
