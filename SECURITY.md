# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it by emailing:
**pmazurki@me.com**

Please do NOT create a public GitHub issue for security vulnerabilities.

You can expect a response within 48 hours.

## Security Measures

This project implements the following security practices:

- **Dependency Auditing**: Automated security audits via `cargo-audit` in CI
- **Dependabot**: Automatic dependency updates for security patches
- **Code Analysis**: Static analysis with Clippy on every commit
- **Secret Scanning**: GitHub secret scanning enabled
- **Signed Releases**: All release artifacts include SHA256 checksums

## Secure Usage

1. **API Tokens**: Always use scoped API tokens instead of Global API Keys
2. **Environment Variables**: Store credentials in `.env` files (excluded from git)
3. **Token Rotation**: Rotate your Cloudflare API tokens regularly
4. **Least Privilege**: Create tokens with minimal required permissions

