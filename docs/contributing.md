---
layout: page
title: Contributing to MCP Probe
permalink: /contributing/
---

# Contributing to MCP Probe

Thank you for your interest in contributing to MCP Probe! This guide will help you get started with contributing to the project, whether you're fixing bugs, adding features, or improving documentation.

## 🎯 Ways to Contribute

- **🐛 Bug Reports** - Help us identify and fix issues
- **✨ Feature Requests** - Suggest new features and improvements
- **💻 Code Contributions** - Fix bugs, implement features, improve performance
- **📚 Documentation** - Improve guides, examples, and API documentation
- **🧪 Testing** - Add tests, improve test coverage, test edge cases
- **🎨 Design** - Improve UI/UX, create graphics, enhance user experience

## 🚀 Quick Start

### 1. Set Up Development Environment

```bash
# Clone the repository
git clone https://github.com/conikeec/mcp-probe.git
cd mcp-probe

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install development dependencies
rustup component add clippy rustfmt

# Build the project
cargo build

# Run tests
cargo test
```

### 2. Verify Setup

```bash
# Run pre-commit checks
./scripts/pre-commit.sh

# Test the CLI
cargo run -- debug --help

# Test with a real MCP server
cargo run -- debug --non-interactive --stdio npx @playwright/mcp@latest
```

### 3. Make Your Changes

1. **Create a branch**: `git checkout -b feature/your-feature-name`
2. **Make changes**: Implement your feature or fix
3. **Test thoroughly**: Run tests and verify functionality
4. **Run pre-commit**: `./scripts/pre-commit.sh`
5. **Commit changes**: Use clear, descriptive commit messages
6. **Push and PR**: Push to your fork and create a pull request

## 🏗️ Project Structure

```
mcp-probe/
├── crates/
│   ├── mcp-core/          # Core MCP protocol implementation
│   │   ├── src/
│   │   │   ├── client.rs      # MCP client implementation
│   │   │   ├── transport/     # Transport layer (stdio, HTTP)
│   │   │   ├── messages/      # MCP message types
│   │   │   └── validation.rs  # Parameter validation
│   │   └── Cargo.toml
│   └── mcp-cli/           # CLI application
│       ├── src/
│       │   ├── main.rs        # CLI entry point
│       │   ├── commands/      # CLI commands
│       │   ├── tui.rs         # Terminal UI
│       │   └── cli.rs         # CLI argument parsing
│       └── Cargo.toml
├── docs/                  # Website and documentation
├── examples/              # Usage examples
├── scripts/               # Development scripts
├── .github/              # GitHub Actions workflows
└── README.md
```

### Key Components

#### mcp-core
- **Transport Layer**: Stdio, HTTP+SSE, HTTP streaming
- **Message Types**: Tools, resources, prompts, sampling
- **Client Implementation**: Connection management, request/response handling
- **Validation**: Parameter validation and transformation

#### mcp-cli
- **Commands**: debug, test, validate, export, config, paths
- **TUI**: Interactive terminal user interface
- **Configuration**: File management and organization

## 🛠️ Development Guidelines

### Code Style

We use standard Rust formatting and linting:

```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Run all pre-commit checks
./scripts/pre-commit.sh
```

### Commit Messages

Use clear, descriptive commit messages following conventional commits:

```
type(scope): description

feat(transport): add HTTP streaming support
fix(stdio): resolve message correlation timeout
docs(examples): add CI/CD integration examples
test(validation): add parameter validation tests
```

Types: `feat`, `fix`, `docs`, `test`, `refactor`, `style`, `perf`, `chore`

### Testing

#### Unit Tests
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_stdio_transport

# Run tests with output
cargo test -- --nocapture
```

#### Integration Tests
```bash
# Test with real MCP servers
cargo run -- debug --non-interactive --stdio npx @playwright/mcp@latest
cargo run -- test --stdio python examples/simple_server.py --report
```

#### Test Coverage
```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html
```

### Documentation

#### Code Documentation
- Use rustdoc comments for public APIs
- Include examples in documentation
- Document error conditions and edge cases

```rust
/// Validates and transforms MCP tool parameters.
///
/// This function automatically fixes common parameter issues:
/// - Adds protocol prefixes to URLs (`www.google.com` → `https://www.google.com`)
/// - Coerces string numbers to integers/floats
/// - Validates required fields and types
///
/// # Arguments
///
/// * `schema` - JSON schema for the tool parameters
/// * `params` - Raw parameter values to validate
///
/// # Returns
///
/// A `ValidationResult` containing validated parameters and any transformations applied.
///
/// # Examples
///
/// ```rust
/// let result = validator.validate_parameters(&schema, &params)?;
/// assert!(result.is_valid);
/// assert_eq!(result.transformations.len(), 1); // URL was prefixed
/// ```
pub fn validate_parameters(&self, schema: &Value, params: &Value) -> McpResult<ValidationResult> {
    // Implementation...
}
```

#### User Documentation
- Update relevant documentation files
- Add examples for new features
- Include troubleshooting information

## 🐛 Bug Reports

### Before Reporting

1. **Search existing issues** - Check if the bug is already reported
2. **Test with latest version** - Ensure you're using the current release
3. **Isolate the problem** - Create a minimal reproduction case

### Bug Report Template

```markdown
**Bug Description**
A clear description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Run command: `mcp-probe debug --stdio python server.py`
2. See error: ...

**Expected Behavior**
What you expected to happen.

**Actual Behavior**
What actually happened.

**Environment**
- OS: [e.g., macOS 14.0, Ubuntu 22.04]
- MCP Probe version: [e.g., 0.2.4]
- Rust version: [e.g., 1.75.0]
- MCP server: [e.g., @playwright/mcp@latest]

**Additional Context**
- Logs: Include relevant log files from `~/.mcp-probe/logs/`
- Configuration: Include relevant config files
- Screenshots: If applicable
```

## ✨ Feature Requests

### Before Requesting

1. **Check existing requests** - Look for similar feature requests
2. **Consider alternatives** - Can existing features solve your problem?
3. **Think about scope** - Is this a general-purpose feature?

### Feature Request Template

```markdown
**Feature Description**
A clear description of the feature you'd like to see.

**Use Case**
Describe the problem this feature would solve.

**Proposed Solution**
How you envision this feature working.

**Alternatives Considered**
Other approaches you've considered.

**Additional Context**
Any other context, mockups, or examples.
```

## 🔄 Pull Request Process

### 1. Preparation

- **Fork the repository** and create a feature branch
- **Discuss large changes** in an issue first
- **Follow coding standards** and run pre-commit checks
- **Write tests** for new functionality
- **Update documentation** as needed

### 2. Pull Request Guidelines

#### Title and Description
- Use descriptive titles: `feat(transport): add WebSocket transport support`
- Include issue references: `Fixes #123`, `Closes #456`
- Describe changes, motivation, and impact

#### Checklist
- [ ] Code follows project style guidelines
- [ ] Tests pass: `cargo test`
- [ ] Pre-commit checks pass: `./scripts/pre-commit.sh`
- [ ] Documentation updated (if applicable)
- [ ] Changelog updated (for significant changes)

### 3. Review Process

1. **Automated checks** - CI must pass
2. **Code review** - Maintainers will review your code
3. **Discussion** - Address feedback and suggestions
4. **Approval** - Merge when approved

### 4. After Merge

- **Delete your branch** after successful merge
- **Update your fork** with the latest changes
- **Consider follow-up improvements** or documentation

## 🧪 Testing Guidelines

### Test Types

#### Unit Tests
Test individual functions and components:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_auto_prefixing() {
        let validator = ParameterValidator::new(true);
        let result = validator.transform_url_field("www.google.com");
        assert_eq!(result, "https://www.google.com");
    }

    #[tokio::test]
    async fn test_stdio_transport_connection() {
        let config = TransportConfig::stdio("echo", &["hello"]);
        let mut transport = StdioTransport::new(config);
        assert!(transport.connect().await.is_ok());
    }
}
```

#### Integration Tests
Test complete workflows:

```rust
#[tokio::test]
async fn test_full_debug_workflow() {
    let mut client = McpClient::with_defaults(
        TransportConfig::stdio("npx", &["@playwright/mcp@latest"])
    ).await?;
    
    let server_info = client.connect(client_info).await?;
    assert!(server_info.capabilities.tools.is_some());
    
    let tools = client.list_tools().await?;
    assert!(!tools.is_empty());
}
```

### Test Data

Create realistic test scenarios:

```rust
// Use real MCP server examples
const PLAYWRIGHT_TOOLS: &[&str] = &[
    "browser_navigate",
    "browser_click", 
    "browser_type",
    "browser_take_screenshot"
];

// Test edge cases
#[test]
fn test_url_edge_cases() {
    let test_cases = vec![
        ("www.google.com", "https://www.google.com"),
        ("localhost:3000", "http://localhost:3000"),
        ("https://already-prefixed.com", "https://already-prefixed.com"),
        ("invalid-url", "invalid-url"), // Should not transform
    ];
    
    for (input, expected) in test_cases {
        assert_eq!(transform_url(input), expected);
    }
}
```

## 📊 Performance Considerations

### Benchmarking

Use criterion for performance testing:

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_parameter_validation(c: &mut Criterion) {
    let validator = ParameterValidator::new(true);
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "url": {"type": "string"}
        }
    });
    
    c.bench_function("validate_parameters", |b| {
        b.iter(|| validator.validate_parameters(&schema, &params))
    });
}

criterion_group!(benches, benchmark_parameter_validation);
criterion_main!(benches);
```

### Optimization Guidelines

- **Avoid unnecessary allocations** in hot paths
- **Use appropriate data structures** (HashMap vs BTreeMap)
- **Consider async vs sync** for I/O operations
- **Profile before optimizing** using `cargo flamegraph`

## 🔧 Development Tools

### Recommended Tools

```bash
# Code quality
cargo install cargo-clippy cargo-fmt

# Testing
cargo install cargo-tarpaulin  # Coverage
cargo install cargo-nextest    # Fast test runner

# Performance
cargo install cargo-flamegraph # Profiling
cargo install criterion        # Benchmarking

# Debugging
cargo install cargo-expand     # Macro expansion
```

### IDE Setup

#### VS Code
Recommended extensions:
- rust-analyzer
- CodeLLDB (debugging)
- Better TOML
- GitLens

#### Configuration
```json
{
    "rust-analyzer.check.command": "clippy",
    "rust-analyzer.cargo.features": "all",
    "editor.formatOnSave": true
}
```

## 🚀 Release Process

### Version Bumping

Use our automated version management:

```bash
# Patch version (bug fixes)
./scripts/version-bump.sh patch

# Minor version (new features)
./scripts/version-bump.sh minor

# Major version (breaking changes)
./scripts/version-bump.sh major

# Specific version
./scripts/version-bump.sh 1.0.0
```

### Changelog

Update `CHANGELOG.md` for significant changes:

```markdown
## [0.2.5] - 2024-01-15

### Added
- WebSocket transport support
- Configuration validation command
- Interactive parameter prompting

### Fixed
- Stdio transport timeout issues
- Memory leak in HTTP client
- Configuration file parsing

### Changed
- Improved error messages
- Updated dependencies
- Enhanced documentation
```

## 🤝 Community Guidelines

### Code of Conduct

We follow the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct):

- **Be friendly and patient**
- **Be welcoming and inclusive**
- **Be respectful**
- **Choose your words carefully**

### Communication Channels

- **GitHub Issues** - Bug reports and feature requests
- **GitHub Discussions** - General questions and community discussion
- **Pull Requests** - Code review and collaboration

### Getting Help

1. **Check documentation** - README, docs/, examples/
2. **Search existing issues** - Someone might have had the same problem
3. **Ask in discussions** - For general questions
4. **Create an issue** - For specific bugs or feature requests

## 🏆 Recognition

Contributors are recognized in:
- **README.md** - List of contributors
- **Releases** - Contributor mentions in release notes
- **Documentation** - Author attribution where appropriate

### Contributor Levels

- **First-time contributor** - Welcome and guidance provided
- **Regular contributor** - Recognition in project documentation
- **Maintainer** - Commit access and release responsibilities

## 📚 Resources

### Rust Resources
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Async Programming in Rust](https://rust-lang.github.io/async-book/)

### MCP Resources
- [Model Context Protocol Specification](https://spec.modelcontextprotocol.io/)
- [MCP SDKs](https://github.com/modelcontextprotocol)
- [Example MCP Servers](https://github.com/modelcontextprotocol/servers)

### Testing Resources
- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Tokio Testing](https://tokio.rs/tokio/topics/testing)
- [Criterion Benchmarking](https://github.com/bheisler/criterion.rs)

---

## 🎯 Getting Started Checklist

Ready to contribute? Here's your checklist:

- [ ] ⭐ Star the repository
- [ ] 🍴 Fork the repository  
- [ ] 💻 Set up development environment
- [ ] ✅ Run `./scripts/pre-commit.sh` successfully
- [ ] 📖 Read this contributing guide
- [ ] 🎯 Find an issue to work on or create one
- [ ] 🚀 Make your first contribution!

**Questions?** Don't hesitate to ask in [GitHub Discussions](https://github.com/conikeec/mcp-probe/discussions) or create an issue.

---

*Thank you for contributing to MCP Probe! Together, we're making MCP debugging better for everyone.* 🎉 