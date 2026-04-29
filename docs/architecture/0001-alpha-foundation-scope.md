# Alpha Foundation Scope

**Date**: 2026-04-30
**Status**: Active
**Milestone**: 26.1.0-alpha.1
**Phase**: 0-governance

## Objective

Define the boundaries of the alpha release (26.1.0-alpha.1) to prevent scope explosion and keep the project focused on reusable foundations.

## In-Scope

The alpha focuses on **reusable foundations**, not vertical market solutions. The following areas are in-scope:

### Kernel Core
- Boot process and initialization
- Process/micro-service management
- Local-first data handling primitives
- Minimal syscall interface

### Storage Foundation
- SQLite as primary local storage engine
- FTS5 full-text search infrastructure
- Backup and restore primitives
- Database maintenance commands

### Security Foundation
- Role-Based Access Control (RBAC) policy framework
- Development identity and authorization interfaces
- Security policy enforcement points

### Operations Foundation
- Structured tracing infrastructure
- Metrics collection
- Health check endpoints
- Timeline and relationship neighborhood services

### Documentation
- Architecture decision records (this document series)
- Developer documentation
- Operations documentation
- Demo seed data and scenarios

## Out-of-Scope

The following items are **explicitly out-of-scope** for the alpha release. Feature requests for these areas should be rejected with a pointer to this document:

### Vertical Market Features
- Industry-specific workflows (healthcare, finance, education, etc.)
- Domain-specific data models beyond generic primitives
- Specialized UI/UX for specific user personas

### Advanced Distributed Systems
- Multi-node clustering
- Consensus algorithms
- Distributed consensus or coordination
- Cloud synchronization (beyond local-first export/import)

### Production Hardening
- High availability configurations
- Live migration of services
- Advanced performance optimization
- Enterprise integration patterns

### External Integrations
- Third-party API integrations
- Cloud provider SDKs
- External authentication providers (OAuth, SAML, etc.)
- Social login or identity federation

## Scope Evaluation Criteria

When evaluating new feature requests, ask:

1. **Is this a reusable foundation?** If it's specific to a vertical market, reject.
2. **Does this block other in-scope work?** If not, defer to post-alpha.
3. **Can this be built as a module on top of foundations?** If yes, it's out-of-scope for alpha.

## Rejection Template

When rejecting out-of-scope requests, use this template:

```
Thank you for the suggestion. This feature falls outside the alpha scope defined in 
[docs/architecture/0001-alpha-foundation-scope.md](../docs/architecture/0001-alpha-foundation-scope.md). 

The alpha (26.1.0-alpha.1) focuses on reusable foundations, not vertical market solutions. 
This feature appears to be [vertical-specific / production-hardening / distributed-systems / integration].

We plan to revisit this area after the alpha foundations are stable. Feel free to open an 
issue labeled `post-alpha` to track it for future consideration.
```
