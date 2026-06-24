# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | ✅        |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly:

1. **Do NOT open a public issue**
2. Email: flurion@tuta.io
3. Or DM on Discord: blankhtml.page

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

## Response Timeline

- **Acknowledgment**: within 48 hours
- **Assessment**: within 1 week
- **Fix**: depends on severity, typically within 2 weeks

## Scope

This project interacts with:
- Discord IPC socket (local only)
- Roblox API (read-only, public endpoints)

No authentication data, tokens, or user credentials are handled.

## Disclosure

Once a fix is released, we will:
- Credit the reporter (unless they prefer anonymity)
- Publish a security advisory if the vulnerability is severe

## AI Usage Disclosure

This project is fully or partially built using AI assistants. All AI-generated code is reviewed by a human maintainer before merging. Security-critical changes receive additional scrutiny. If you find an issue in AI-generated code, report it the same way as any other vulnerability.
