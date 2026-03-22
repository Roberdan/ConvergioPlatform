# Security Policy

<!-- Copyright (c) 2026 Roberto D'Angelo. Licensed under MPL-2.0. -->

See also: [LEGAL_NOTICE.md](LEGAL_NOTICE.md) | [CONSTITUTION.md](CONSTITUTION.md) (Article II — Safety)

## Experimental Software Notice

Convergio Platform is **experimental software** provided for research and educational purposes only. It is not intended for production use or handling of sensitive data.

## Security Disclaimers

- **No Security Guarantees**: This software is provided "as-is" without security warranties
- **Experimental Nature**: Security features may be incomplete or contain vulnerabilities
- **Educational Purpose**: Designed for learning and research, not production deployment
- **No Sensitive Data**: Do not use with confidential, proprietary, or sensitive information

## Reporting Security Issues

If you discover a security vulnerability, please:

1. **Email**: roberdan@fightthestroke.org
2. **Subject**: "Security Issue - Convergio Platform"
3. **Include**:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact assessment
   - Your contact information

Do not open public issues for security vulnerabilities.

## Security Best Practices

When experimenting with Convergio Platform:

### Do

- Use in isolated, non-production environments only
- Review all agent code before deployment
- Understand that agents may access files and make web requests
- Keep your Claude Code environment updated
- Use test data only

### Do Not

- Use with sensitive or confidential information
- Deploy in production environments
- Share API keys or credentials with agents
- Assume any security protections exist
- Use for processing personal or proprietary data

## Current Security Considerations

### Agent Capabilities

- Agents can read/write files within their tool access scope
- Some agents can make web requests
- Agents can execute coordinated workflows via the IPC bus (daemon :8420)
- All activity should be considered logged

### Daemon Surface

- HTTP/WS API exposed on :8420 (local or Tailscale mesh only)
- HMAC-SHA256 authenticates all mesh node messages
- No unauthenticated remote access is permitted by design

### Known Limitations

- No built-in sandboxing beyond Claude Code's default protections
- No encrypted storage of agent interactions
- No audit logging beyond standard Claude Code logs and daemon traces
- No access controls beyond tool-level restrictions

## Supported Versions

| Version | Security Updates |
|---------|-----------------|
| Current main branch | Best effort, experimental |
| Older versions | Not supported |

## Constitution Reference

Security practices for this project are governed in part by CONSTITUTION.md Article II (Safety). Contributors and users are expected to read and follow those principles.

## Updates

This security policy may be updated as the project evolves. Check back regularly for changes.

## Contact

For security-related questions: roberdan@fightthestroke.org

---

**Remember: This is experimental software. Use at your own risk and never with sensitive data.**
