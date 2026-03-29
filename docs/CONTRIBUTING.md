# Contributing to Good4NCU

Thank you for your interest in contributing to Good4NCU!

## Branch Strategy

We use a simple branching model:

- **`main`** - Production-ready code, protected branch
- **`feat/<description>`** - New features (e.g., `feat/user-authentication`)
- **`fix/<description>`** - Bug fixes (e.g., `fix/cors-origin-validation`)

All feature and fix branches should be based off `main`.

## Conventional Commits

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

| Type | Description |
|------|-------------|
| `feat` | A new feature |
| `fix` | A bug fix |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `docs` | Documentation only changes |
| `style` | Changes that do not affect code meaning (formatting) |
| `test` | Adding or correcting tests |
| `chore` | Changes to build process, CI, or auxiliary tools |

### Examples

```
feat(api): add user registration endpoint

fix(auth): handle expired JWT tokens gracefully

refactor(db): extract connection pooling to separate module

docs(readme): update installation instructions

feat(chat): implement real-time messaging with WebSocket
```

### Scope (Optional)

Scopes are specific to the codebase structure:

- `api` - API routes and handlers
- `auth` - Authentication and authorization
- `llm` - LLM provider implementations
- `db` - Database schema and queries
- `cli` - Interactive CLI
- `mobile` - Flutter mobile app
- `ci` - GitHub Actions and DevOps

## Pull Request Process

### PR Requirements

1. **Branch up to date** - Rebase onto latest `main` before submitting PR
2. **Tests pass** - All `cargo test` must pass
3. **Lint passes** - `cargo clippy -- -D warnings` must pass with no warnings
4. **Format correct** - `cargo fmt` must show no changes needed
5. **Sign commits** - GPG signing recommended but not required

### PR Description Template

```markdown
## Summary
Brief description of the change.

## Type
- [ ] Feature
- [ ] Bug Fix
- [ ] Refactor
- [ ] Documentation
- [ ] Other

## Test Plan
Steps to test the change:
1. ...
2. ...

## Checklist
- [ ] Code follows project style
- [ ] Tests added/updated
- [ ] Documentation updated
```

### Review Requirements

- At least 1 approval required to merge to `main`
- Reviews should check: correctness, style, test coverage, documentation
- Approvals do not expire but should be re-requested if significant changes are made

## Development Setup

1. Copy `.env.example` to `.env` and fill in required values
2. Run `cargo build` to build the project
3. Run `cargo check` for type checking
4. Run `cargo test` to run tests

## Getting Help

- Open an issue for bugs or feature requests
- Check `CLAUDE.md` for project architecture details
- Check `AGENTS.md` for AI agent implementation details
