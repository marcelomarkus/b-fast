# Contributing to B-FAST

Thank you for your interest in contributing to B-FAST! We welcome contributions from the community and are excited to see what you'll bring to the project.

## 🌟 Philosophy

> "Knowledge is the only wealth that grows when we share it"

B-FAST is built on the principle of open knowledge sharing. Every contribution, no matter how small, helps make high-performance serialization accessible to more developers.

## 🚀 Getting Started

### Prerequisites
- **Rust** 1.70+ (for core library)
- **Python** 3.8+ with uv or pip
- **Node.js** 14+ (for TypeScript client)
- **Git** for version control

### Development Setup

1. **Fork and clone the repository:**
   ```bash
   git clone https://github.com/your-username/b-fast.git
   cd b-fast
   ```

2. **Set up Python environment:**
   ```bash
   uv sync --extra dev
   # ou
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
   python -m pytest tests/
   cd client-ts && npm run build
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
   - `feat/*` - New features or improvements (e.g., `feat/uuid-support`)
   - `fix/*` - Bug fixes (e.g., `fix/memory-leak`)
   - `docs/*` - Documentation only (e.g., `docs/api-examples`)
   
   **Note:** CI automatically runs on `feat/*` and `fix/*` branches. Documentation-only changes (`docs/*`) don't trigger CI.

2. **Make your changes** following our coding standards

3. **Run the full test suite:**
   ```bash
   # Python tests
   python -m pytest tests/ -v
   
   # Code formatting
   python -m black .
   python -m ruff check .
   
   # TypeScript build
   cd client-ts && npm run build
   ```

4. **Commit with descriptive messages:**
   ```bash
   git commit -m "feat: add support for UUID serialization
   
   - Implement UUID type (tag 0x80) in Rust encoder
   - Add UUID parsing in TypeScript decoder
   - Include tests for UUID round-trip serialization"
   ```

## 📝 Contribution Types

### 🐛 Bug Reports
- Use the bug report template
- Include minimal reproduction case
- Specify Python/Node.js/Rust versions
- Include error messages and stack traces

### ✨ Feature Requests
- Use the feature request template
- Explain the use case and benefits
- Consider backward compatibility
- Discuss performance implications

### 🔧 Code Contributions
- **Core Rust library:** `src/lib.rs`, `src/errors.rs`
- **Python bindings:** `python/b_fast/`
- **TypeScript client:** `client-ts/`
- **Documentation:** `docs/`, `README.md`
- **Tests:** `tests/`

### 📚 Documentation
- API documentation improvements
- Usage examples and tutorials
- Performance optimization guides
- Translation to other languages

## 🎯 Coding Standards

### Python
- **Formatting:** Black (line length 88)
- **Linting:** Ruff with project configuration
- **Type hints:** Required for public APIs
- **Tests:** pytest with descriptive names

### Rust
- **Formatting:** `cargo fmt`
- **Linting:** `cargo clippy`
- **Documentation:** Rustdoc comments for public APIs
- **Safety:** Minimize unsafe code, document when necessary

### TypeScript
- **Formatting:** Prettier (via npm scripts)
- **Linting:** ESLint configuration
- **Types:** Strict TypeScript, no `any` in public APIs
- **Compatibility:** ES2020+ for modern environments

## 🧪 Testing Guidelines

### Test Categories
- **Unit tests:** Individual function/method testing
- **Integration tests:** Component interaction testing
- **Performance tests:** Benchmark critical paths
- **Compatibility tests:** Cross-platform validation

### Test Requirements
- All new features must include tests
- Bug fixes must include regression tests
- Performance changes need benchmark comparisons
- Breaking changes require migration guides

## 📋 Pull Request Process

1. **Ensure your PR:**
   - Has a clear, descriptive title
   - References related issues (`Fixes #123`)
   - Includes tests for new functionality
   - Updates documentation if needed
   - Passes all CI checks

2. **PR Review Process:**
   - Maintainers will review within 48 hours
   - Address feedback promptly
   - Keep discussions respectful and constructive
   - Squash commits before merge if requested

3. **Merge Requirements:**
   - All tests passing
   - Code review approval
   - No merge conflicts
   - Documentation updated

## 🏷️ Issue Labels

### Automatic Labels (added by CI)
- `feat` - Feature branch (automatically added to PRs from `feat/*` branches)
- `fix` - Bug fix branch (automatically added to PRs from `fix/*` branches)
- `docs` - Documentation branch (automatically added to PRs from `docs/*` branches)
- `approved-N` - Number of approvals (e.g., `approved-1`, `approved-2`)

### Manual Labels
- `bug` - Something isn't working
- `enhancement` - New feature or improvement
- `documentation` - Documentation improvements
- `good first issue` - Good for newcomers
- `help wanted` - Extra attention needed
- `python` - Python-specific issues
- `typescript` - TypeScript client issues
- `rust` - Rust core library issues
- `performance` - Performance-related changes

## 🤝 Community Guidelines

### Be Respectful
- Use inclusive language
- Respect different perspectives and experiences
- Focus on constructive feedback
- Help newcomers feel welcome

### Be Collaborative
- Share knowledge and resources
- Credit others' contributions
- Ask questions when unsure
- Offer help to other contributors

### Be Professional
- Keep discussions on-topic
- Avoid personal attacks or harassment
- Respect maintainers' time and decisions
- Follow the code of conduct

## 🆘 Getting Help

- **Questions:** Open a discussion or issue
- **Documentation:** https://marcelomarkus.github.io/b-fast/
- **Chat:** GitHub Discussions for real-time help
- **Examples:** See `main.py` and client examples

## 📄 License

By contributing to B-FAST, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to B-FAST! Together, we're making high-performance serialization accessible to everyone. 🚀
