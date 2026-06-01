# Project Templates

A repeatable project documentation system for structured software development.

## Quick Start

```bash
# Copy the entire .project folder to your repo root
cp -r /path/to/project-templates/.project /your/project/root/
```

## Directory Structure

```
your-project/
├── .project/
│   ├── prd.md           # Product Requirements Document
│   ├── tech-stack.md    # Technology choices and rationale
│   ├── build-plan.md    # Task tracking with phases
│   └── changelog.md     # Version history
└── [project files]
```

## Files

| File | Purpose |
|------|---------|
| `prd.md` | Define what you're building, features, and specs |
| `tech-stack.md` | Document technology choices with rationale |
| `build-plan.md` | Track tasks by phase with progress indicators |
| `changelog.md` | Log changes and version history |

## Build Plan Features

### Status Emojis
| Icon | Status | Description |
|------|--------|-------------|
| ⬜ | Not Started | Task has not begun |
| 🔄 | In Progress | Currently being worked on |
| ✅ | Completed | Task finished |
| ⛔ | Blocked | Cannot proceed due to external dependency |
| ⚠️ | Has Blockers | Waiting on another task |
| 🔍 | In Review | Pending review/approval |
| 🚫 | Skipped | Intentionally not doing |
| ⏸️ | Deferred | Postponed to later phase/sprint |

### Progress Visualization
```
Phase 1: Setup     [████████████████████] 100%  ✅
Phase 2: Core      [████████████░░░░░░░░]  60%  🔄
Phase 3: Data      [░░░░░░░░░░░░░░░░░░░░]   0%  ⬜
```

## Workflow

1. **Start Project** - Copy templates to `.project/`
2. **Define Requirements** - Fill out `prd.md`
3. **Choose Stack** - Document in `tech-stack.md`
4. **Plan Tasks** - Break down work in `build-plan.md`
5. **Build** - Update status after each task
6. **Log Changes** - Update `changelog.md` at milestones

## Build Discipline

After completing each task:
1. Run build command
2. Fix any warnings/errors
3. Mark task as ✅
4. Update progress summary
5. Commit changes
