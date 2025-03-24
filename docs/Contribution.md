# Contributing to MCPI

## Getting Started

### Prerequisites
- Rust 1.85.1 or newer
- Cargo
- Git
- Basic understanding of WebSocket and JSON-RPC protocols

### Development Setup
1. Fork the repository
2. Clone your fork
3. Create a feature branch
   ```bash
   git checkout -b feature/my-awesome-contribution
   ```

## Contribution Types

### Code Contributions
- Plugin Development
- Core Protocol Improvements
- Performance Optimizations
- Bug Fixes

### Documentation
- Protocol Specification Updates
- README Improvements
- Examples and Tutorials

## Contribution Process

1. **Issue Tracking**
   - Check existing issues before starting
   - Create an issue describing your proposed changes

2. **Development Guidelines**
   - Follow Rust best practices
   - Write comprehensive tests
   - Maintain code readability
   - Document new features/changes

3. **Submitting a Pull Request**
   - Ensure all tests pass
   - Update documentation
   - Describe changes in PR description
   - Link related issues

## Plugin Development Guide

### Creating a New Plugin
1. Implement `McpPlugin` trait
2. Define supported operations
3. Provide input schema
4. Implement operation logic
5. Add tests

### Best Practices
- Keep plugins focused and modular
- Handle errors gracefully
- Provide clear documentation
- Consider performance implications

## Code of Conduct
- Be respectful
- Collaborate constructively
- Welcome diverse perspectives

## Licensing
Contributions are made under the MIT License