# [Project Name] - Tech Stack

> **Document Location:** `.project/tech-stack.md`
>
> This document outlines the technology choices and rationale for the project.
> All technology decisions should be documented here with reasoning.

---

## Stack Overview

```
┌─────────────────────────────────────────────────┐
│                   Frontend                       │
│  [Framework] + [UI Library] + [State Mgmt]      │
├─────────────────────────────────────────────────┤
│                    Backend                       │
│  [Runtime] + [Framework] + [API Style]          │
├─────────────────────────────────────────────────┤
│                   Data Layer                     │
│  [Database] + [ORM/Query Builder] + [Cache]     │
├─────────────────────────────────────────────────┤
│                Infrastructure                    │
│  [Hosting] + [CI/CD] + [Monitoring]             │
└─────────────────────────────────────────────────┘
```

---

## Core Technologies

### Language & Runtime

| Technology | Version | Purpose |
|------------|---------|---------|
| [Language] | x.x.x | Primary language |
| [Runtime] | x.x.x | Execution environment |

**Rationale:**
- [Why this choice?]
- [What alternatives were considered?]

---

### Framework

| Technology | Version | Purpose |
|------------|---------|---------|
| [Framework] | x.x.x | Application framework |

**Rationale:**
- [Why this choice?]

---

### Database

| Technology | Version | Purpose |
|------------|---------|---------|
| [Database] | x.x.x | Primary data store |
| [ORM/Driver] | x.x.x | Database access |

**Rationale:**
- [Why this choice?]

**Schema Location:** `[path/to/schema]`

---

## Dependencies

### Production Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| [package-1] | ^x.x.x | [Purpose] |
| [package-2] | ^x.x.x | [Purpose] |
| [package-3] | ^x.x.x | [Purpose] |

### Development Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| [dev-package-1] | ^x.x.x | [Purpose] |
| [dev-package-2] | ^x.x.x | [Purpose] |

---

## Build & Tooling

### Build System

| Tool | Version | Purpose |
|------|---------|---------|
| [Build Tool] | x.x.x | Build/bundle |
| [Task Runner] | x.x.x | Automation |

### Development Tools

| Tool | Purpose |
|------|---------|
| [Linter] | Code quality |
| [Formatter] | Code style |
| [Type Checker] | Type safety |

### Build Commands

```bash
# Development
[dev command]

# Production build
[build command]

# Testing
[test command]

# Linting
[lint command]
```

---

## Architecture Patterns

### Code Organization

```
project-root/
├── .project/           # Project documentation
│   ├── prd.md
│   ├── tech-stack.md
│   ├── build-plan.md
│   └── changelog.md
├── src/
│   ├── [module-1]/     # Feature module
│   ├── [module-2]/     # Feature module
│   ├── shared/         # Shared utilities
│   └── [entry-point]   # Application entry
├── tests/
│   ├── unit/
│   └── integration/
└── [config files]
```

### Design Patterns Used

| Pattern | Where Used | Purpose |
|---------|------------|---------|
| [Pattern 1] | [Location] | [Why] |
| [Pattern 2] | [Location] | [Why] |

---

## Environment Configuration

### Required Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `DATABASE_URL` | Database connection string | Yes |
| `API_KEY` | External API key | Yes |
| `DEBUG` | Enable debug mode | No |

### Configuration Files

| File | Purpose |
|------|---------|
| `.env` | Environment variables (not committed) |
| `.env.example` | Template for environment variables |
| `[config file]` | Application configuration |

---

## External Services

### APIs & Integrations

| Service | Purpose | Documentation |
|---------|---------|---------------|
| [Service 1] | [Purpose] | [Link] |
| [Service 2] | [Purpose] | [Link] |

### Third-Party Services

| Service | Purpose | Account Required |
|---------|---------|------------------|
| [Service] | [Purpose] | Yes/No |

---

## Security Considerations

### Authentication
- [Authentication method]

### Data Protection
- [Encryption at rest/transit]
- [Sensitive data handling]

### Dependencies
- [Vulnerability scanning approach]
- [Update policy]

---

## Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| [Metric 1] | [Target] | [How measured] |
| [Metric 2] | [Target] | [How measured] |

---

## Decision Log

| Date | Decision | Rationale | Alternatives Considered |
|------|----------|-----------|------------------------|
| YYYY-MM-DD | [Decision] | [Why] | [Options] |

---

*Last updated: YYYY-MM-DD*
