# CI/CD Guide — kTemplate

## Overview

Year-prefixed semver (`YY.Major.Minor.Patch[hotfix]`) with automated version bumping, issue-to-branch automation, and GitHub release management.

## Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `version-validation.yml` | PR to `main` or `release` | Validate `VERSION` file format and uniqueness vs latest tag |
| `version-bump.yml` | PR merged to `main` / direct push with `hotfix:` | Auto-increment version in `VERSION` file |
| `manual-version-bump.yml` | `workflow_dispatch` | Manual bump (major/minor/patch) or year rollover |
| `release.yml` | PR merged to `release` / push to `release` | Sync VERSION from main, create Git tag + GitHub Release |
| `issue-branch-handler.yml` | Issue labeled `task`, `feature`, or `bug` | Create branch + draft PR + sub-issue parent tracking |
| `deploy-docs.yml` | Push to `main` touching `docs/visual-explainer/**`, or `workflow_dispatch` | Build gallery index and push to `visual-explainer` branch for GitHub Pages |

## VERSION File

Located at the repository root. Single source of truth for the version.

```text
26.0.0.0
```

**Format**: `YY.Major.Minor.Patch[hotfix]`
- YY = two-digit year
- Major = tasks / large work units (resets Minor, Patch on increment)
- Minor = features (resets Patch on increment)
- Patch = bug fixes
- hotfix = emergency suffix letters (a, b, … z, A …) — rare

## Branch Strategy

```
main              ← protected
release           ← production releases
visual-explainer  ← GitHub Pages (managed by CI, do not commit to directly)
task/{n}-...      ← large work units (branch from main)
feature/{n}-…     ← features (branch from task or main)
bug/{n}-…         ← bug fixes (branch from feature, task, or main)
```

Sub-issues automatically branch from their parent issue's branch via `issue-branch-handler.yml`.

## Version Bumping Rules

| Branch type merged → main | Bump |
|---------------------------|------|
| `task/*` | Major +1, Minor → 0, Patch → 0 |
| `feature/*` | Minor +1, Patch → 0 |
| `bug/*` or `hotfix/*` | Patch +1 |
| Direct push with `hotfix:` in message | Suffix (a, b, … z, A …) |

On year change, the next bump resets to `YY.0.0.0` regardless of bump type.

## Issue Hierarchy

```
Task (task label)
└── Feature (feature label, sub-issue of Task)
    └── Bug (bug label, sub-issue of Feature or Task)
```

- Create a Task → branch `task/{n}-...` auto-created from `main`, draft PR opened
- Add Feature as sub-issue of Task → branch `feature/{n}-...` auto-created from `task/*`
- Add Bug as sub-issue of Feature → branch `bug/{n}-...` auto-created from `feature/*`

## Year Rollover

Run **Manual Version Bump** from Actions → workflow_dispatch, select `year-rollover`:
- Resets to `YY.0.0.0` when current year differs from VERSION year
- No-op if already on current year

## Release Process

1. Create PR from `main` → `release` (use **Rebase**)
2. `version-validation.yml` checks format and uniqueness vs latest tag
3. After merge, `release.yml` creates tag `v{VERSION}` and a GitHub Release

## Setting Up the Release Branch

```bash
git checkout main
git pull origin main
git checkout -b release
git push origin release
```

## Branch Protection (configure via GitHub settings)

### main
- Require PR before merging (1 approval)
- Dismiss stale reviews
- Required status checks: `validate-version`
- Require linear history
- No force pushes, no deletions

### release
- Require PR before merging (1 approval)
- Required status checks: `validate-version`
- Require linear history
- No force pushes, no deletions

### visual-explainer
- No branch protection needed — CI writes to it directly
- Do not commit to this branch manually

## Labels to Create

Create these labels in the repo for the workflows to trigger correctly:

| Label | Color | Purpose |
|-------|-------|---------|
| `task` | `#0075ca` | Top-level work unit |
| `feature` | `#a2eeef` | Feature within a task |
| `bug` | `#d73a4a` | Bug fix |
| `implementation` | `#e4e669` | Auto-added to task PRs |
| `addition` | `#0e8a16` | Auto-added to feature PRs |
| `fix` | `#ee0701` | Auto-added to bug PRs |

## GitHub Pages Deployment

`deploy-docs.yml` publishes visual explainer output to GitHub Pages via the `visual-explainer` branch whenever `docs/visual-explainer/**` files change on `main`, or when triggered manually.

### How it works

1. **Build step**: checks out the repo, runs a Python script that walks `docs/visual-explainer/` subdirectories, collects `.html` files, and generates `docs/index.html` — a gallery page grouping files by type (diagrams, slides, reviews, recaps, plans).
2. **Deploy step**: mirrors the contents of `docs/` to the root of the `visual-explainer` branch using a git worktree, then pushes. The branch root becomes what GitHub Pages serves.

### Directory conventions

- `docs/visual-explainer/diagrams/` — output from `generate-web-diagram --publish`
- `docs/visual-explainer/slides/` — output from `generate-slides --publish`
- `docs/visual-explainer/reviews/` — output from `diff-review --publish` and `plan-review --publish`
- `docs/visual-explainer/recaps/` — output from `project-recap --publish`
- `docs/visual-explainer/plans/` — output from `generate-visual-plan --publish`

Only `.html` files in subdirectories are listed. Files placed directly in `docs/visual-explainer/` root are not enumerated.

### Enabling GitHub Pages

1. Go to **Settings → Pages** in the repo.
2. Set **Source** to **Deploy from a branch**.
3. Set **Branch** to `visual-explainer`, folder `/` (root).
4. Push a change to `docs/visual-explainer/**` on `main` (or run the workflow manually) to trigger the first deployment. The `visual-explainer` branch will be created automatically on first deploy.

### Permissions required

The workflow needs `contents: write` (already declared in the YAML) to push to the `visual-explainer` branch.
