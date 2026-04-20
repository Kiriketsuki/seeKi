# CI/CD Guide — SeeKi

## Overview

Year-prefixed semver (`YY.Major.Minor.Patch[hotfix]`) with automated version bumping, issue-to-branch automation, and GitHub release publishing for SeeKi binaries.

## Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `version-validation.yml` | PR to `main` or `release` | Validate `VERSION` file format and uniqueness vs latest tag |
| `version-bump.yml` | PR merged to `main` / direct push with `hotfix:` | Auto-increment version in `VERSION` file |
| `manual-version-bump.yml` | `workflow_dispatch` | Manual bump (major/minor/patch) or year rollover |
| `release.yml` | Push to `release`, `task/*`, `feature/*`, `bug/*`, `hotfix/*`, or `workflow_dispatch` | Build 3 platform binaries, publish 6 release assets, stable on `release`, prerelease on supported work branches |
| `issue-branch-handler.yml` | Issue labeled `task`, `feature`, or `bug` | Create branch + draft PR + sub-issue parent tracking |
| `deploy-docs.yml` | Push to `main` touching `docs/**`, or `workflow_dispatch` | Build docs index and push to `docs` branch for GitHub Pages |

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
docs              ← GitHub Pages (managed by CI, do not commit to directly)
task/{n}-...      ← large work units (branch from main)
feature/{n}-…     ← features (branch from task or main)
bug/{n}-…         ← bug fixes (branch from feature, task, or main)
hotfix/{n}-…      ← urgent fixes (branch from main or release-adjacent work)
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

### Stable releases

1. Create PR from `main` → `release` (use **Rebase**)
2. `version-validation.yml` checks format and uniqueness vs latest tag
3. After merge/push to `release`, `release.yml`:
   - syncs `VERSION` from `main` if needed
   - rereads the final `VERSION`
   - builds frontend + release binaries for:
     - `x86_64-unknown-linux-musl`
     - `x86_64-apple-darwin`
     - `aarch64-apple-darwin`
   - creates tag `v{VERSION}`
   - publishes exactly six runtime assets:
     - `seeki-x86_64-linux-musl`
     - `seeki-x86_64-linux-musl.sha256`
     - `seeki-x86_64-darwin`
     - `seeki-x86_64-darwin.sha256`
     - `seeki-aarch64-darwin`
     - `seeki-aarch64-darwin.sha256`

### Prerelease publishing

- Pushes to `task/*`, `feature/*`, `bug/*`, and `hotfix/*` run the same 3-target build matrix.
- `workflow_dispatch` can also publish from the selected ref.
- CI derives a workspace-only prerelease version from `VERSION` + the build date/commit SHA.
- That derived version is written only inside the workflow workspace so the built binary, tag, and release metadata match; it is **not** committed back to the source branch.
- Prereleases publish the same six assets as stable releases, marked as GitHub prereleases.

## Auto-Update Platform Contract

The SeeKi updater currently expects these asset names:

| Platform | Release asset |
|----------|---------------|
| Linux x86_64 | `seeki-x86_64-linux-musl` |
| macOS Intel | `seeki-x86_64-darwin` |
| macOS Apple Silicon | `seeki-aarch64-darwin` |

Each binary must ship with a same-name `.sha256` sidecar because the updater derives the checksum URL by appending `.sha256`.

## macOS quarantine workaround

macOS release binaries are currently unsigned. If Gatekeeper blocks the app after download or update, clear quarantine manually:

```bash
xattr -d com.apple.quarantine /path/to/seeki
```

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

### docs
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

`deploy-docs.yml` publishes documentation to GitHub Pages via the `docs` branch whenever `docs/**` files change on `main`, or when triggered manually.

### How it works

1. **Build step**: checks out the repo, runs a Python script that walks `docs/` subdirectories, collects `.html` files, and generates `docs/index.html` — a landing page grouping files by category.
2. **Deploy step**: mirrors the contents of `docs/` to the root of the `docs` branch using a git worktree, then pushes. The branch root becomes what GitHub Pages serves.

### Enabling GitHub Pages

1. Go to **Settings > Pages** in the repo.
2. Set **Source** to **Deploy from a branch**.
3. Set **Branch** to `docs`, folder `/` (root).
4. Push a change to `docs/**` on `main` (or run the workflow manually) to trigger a deployment.

### Permissions required

The workflow needs `contents: write` (already declared in the YAML) to push to the `docs` branch.
