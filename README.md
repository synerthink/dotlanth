# Dotlanth

A local-first operating system kernel focused on user sovereignty, offline capability, and minimal external dependencies.

## Current Status

This repository is at milestone **26.1.0-alpha.1**, phase **0-governance**. The current scope is foundation setup: licensing, policy files, and initial project structure.

## Goals

- **Local-first**: All core functionality works offline without cloud dependencies
- **User sovereignty**: Users control their data and system behavior
- **Minimal footprint**: Small, auditable kernel with clear boundaries

## License

This project is licensed under the GNU General Public License v3.0 only — see the [LICENSE](LICENSE) file for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development workflow, commit hygiene, and review expectations.

## Architecture

Architecture decisions for the alpha release (26.1.0-alpha.1) are documented as ADRs in [`docs/architecture/`](docs/architecture/):

| ADR | Description |
|-----|-------------|
| [0001-alpha-foundation-scope.md](docs/architecture/0001-alpha-foundation-scope.md) | Alpha scope definition: in-scope/out-of-scope items, reusable foundations focus |
| [0002-modular-monolith-first.md](docs/architecture/0002-modular-monolith-first.md) | Why modular monolith (not microservices), module boundaries, graduation criteria |
| [0003-sqlite-local-first-storage.md](docs/architecture/0003-sqlite-local-first-storage.md) | SQLite as primary storage, local-first principles, storage graduation criteria |

These documents define the project's technical direction and are used to evaluate feature requests. If a proposal conflicts with these decisions, it may be rejected with a pointer to the relevant ADR.

## Security

See [SECURITY.md](SECURITY.md) for vulnerability reporting and current pre-production posture.
