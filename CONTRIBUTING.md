# Contributing to protto

Thank you for your interest in contributing to `protto`! We welcome all kinds of contributions: bug reports, feature requests, documentation improvements, or code contributions.

By contributing, you agree to abide by the [Rust Community Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).

---

## Ways to Contribute

### Reporting Bugs

- Check if the issue already exists in [GitHub Issues](https://github.com/yourusername/protto-rs/issues).
- If not, create a new issue with:
    - A descriptive title.
    - Steps to reproduce the problem.
    - Expected behavior vs actual behavior.
    - Environment details (OS, Rust version, crate version).

### Suggesting Features

- Open a GitHub Issue describing the feature and why it would be useful.
- Include examples or use cases if possible.

### Contributing Code

1. Fork the repository and clone it locally.
2. Create a branch for your feature or bugfix:
   ```bash
   git checkout -b feature/my-feature
   ```
3. Make your changes. Follow these guidelines:
   - Use rustfmt to format your code. 
   - Follow Rust naming conventions. 
   - Add tests for new features or bug fixes. 
   - Update documentation if needed. 

4. Run tests to ensure nothing breaks:
   ```bash
   cargo test

5. Commit your changes with a descriptive message:
   ```bash
   git commit -m "Add feature X"
   ```

6. Push your brnach and open a Pull Request.

### Documentation Improvements
- You can propose updates to README.md, lib.rs Rustdoc, or examples. 
- Ensure examples compile and run correctly (cargo test --doc).

### Code Review
- All contributions go through GitHub Pull Request review. 
- Maintain clarity, safety, and idiomatic Rust practices.

---

### Developer Tools
- Formatting: `cargo fmt`
- Linting: `cargo clippy`
- Testing: `cargo test` and `cargo test --doc`

---

### License
By contributing, you agree that your contributions will be licensed under the crate’s Apache-2.0 license.
