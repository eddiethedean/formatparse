# Security Policy

## Supported Versions

We actively support the following versions with security updates:

| Version | Supported          |
| ------- | ------------------ |
| 0.5.1   | :white_check_mark: |
| < 0.5.1 | :x:                |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security vulnerability, please follow these steps:

### 1. **Do NOT** open a public issue

Do not report security vulnerabilities through public GitHub issues or discussions.

### 2. Email Security Team

Please email security concerns to: **odosmatthews@gmail.com**

Include the following information:
- Type of vulnerability
- Full paths of source file(s) related to the vulnerability
- Location of the affected code (tag/branch/commit or direct URL)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the issue, including how an attacker might exploit the issue

### 3. Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Fix Timeline**: Depends on severity and complexity, but we aim to address critical issues within 30 days

### 4. Disclosure Policy

- We will acknowledge receipt of your vulnerability report
- We will work with you to understand and resolve the issue
- We will provide updates on our progress
- We will notify you when the vulnerability has been fixed
- We will credit you in the security advisory (unless you prefer to remain anonymous)

### 5. Public Disclosure

Please allow us 90 days to address the vulnerability before public disclosure. This gives us time to:
- Investigate and confirm the vulnerability
- Develop a fix
- Test the fix
- Release a patched version
- Notify users of the update

## Security Best Practices for Users

### Input Validation

- Validate and sanitize user input before passing it to formatparse
- Set reasonable limits on input size based on your use case
- Be cautious when parsing patterns from untrusted sources

### Pattern Complexity

- Avoid overly complex patterns from untrusted sources
- Consider pattern complexity when processing user-provided patterns
- Monitor performance when parsing patterns with many fields

### Resource Limits

The library implements the following security limits:
- Maximum pattern length: 10,000 characters
- Maximum input string length: 10,000,000 characters (10MB)
- Maximum number of fields: 100
- Maximum field name length: 200 characters
- Regex compilation timeout: 200ms

If you encounter these limits and need to adjust them for your use case, please open an issue to discuss.

### Regular Expression Denial of Service (ReDoS)

The library includes protection against ReDoS attacks through:
- Regex compilation timeouts
- Pattern complexity validation
- Input size limits

However, users should still:
- Validate patterns from untrusted sources
- Monitor parsing performance
- Use the library's timeout mechanisms

## Security Updates

Security updates will be released as patch versions (e.g., 0.4.2 → 0.4.3) when possible. Critical security issues may require a new minor version.

Subscribe to GitHub releases to be notified of security updates.

## Known Security Considerations

1. **User-Provided Patterns**: Patterns are compiled into regular expressions. Patterns from untrusted sources should be validated.

2. **Large Inputs**: While there are size limits, processing very large inputs may still consume significant resources.

3. **Dependencies**: We regularly audit dependencies for vulnerabilities using `cargo audit` and `pip-audit`. See our CI workflows for automated scanning.

## Security Contact

For security-related questions or to report vulnerabilities:
- **Email**: odosmatthews@gmail.com
- **GPG Key**: (to be added if needed)

Thank you for helping keep formatparse and its users safe!

