## Adversarial Council — Merge epic/1-backend-api-config into main

> Convened: 2026-04-08 | Advocates: 1 | Critics: 1 | Rounds: 3/4 | Motion type: CODE

### Motion
Merge the epic/1-backend-api-config branch into main. This branch adds backend API endpoints, config extensions, setup wizard, CSV export, and database hardening to the SeeKi project.

### Advocate Positions
**ADVOCATE**: The branch delivers real features (config extensions, display config endpoint, setup wizard, CSV export, table filtering), has fixed both prior blocking findings per the fix manifest (u32 OFFSET overflow at `src/db/postgres.rs:287`, invalid CSV sentinel removed at `src/api/mod.rs:302-317`), maintains the SQL injection prevention invariant (identifier validation at `src/db/postgres.rs:169-171`, parameterized queries at `src/db/postgres.rs:281-296`), enforces table access control on all table-scoped endpoints (`src/api/mod.rs:119,178,205`), hardens the setup wizard against TOML injection and blind writes (`src/api/setup.rs:120-184`), and includes 34+ unit and integration tests across all new paths. The prior council's exit criteria are met.

### Critic Positions
**CRITIC**: Raised three gaps. (1) `save_config` success path at `src/api/setup.rs:187` has no end-to-end test — all five tests in the module are rejection tests. (2) CSV export delivers silently truncated data with `200 OK` on mid-stream errors (`src/api/mod.rs:315-317`) — non-technical users receive incomplete data with no signal. (3) CORS prefix matching at `src/main.rs:47` matches broader origins than intended. CRITIC ultimately conceded all three as non-blocking: Gap 3 on scope grounds (fix manifest classified it as informational), Gap 1 on indirect coverage (component functions are tested, integration test gap is pre-existing infrastructure constraint), Gap 2 on prior mandate (fix manifest explicitly chose option (a) knowing the tradeoff).

### Questioner Findings
No questioner in this council (1v1 format).

### Key Conflicts
- **save_config test coverage** — CRITIC argued the success path is "completely untested"; ADVOCATE showed indirect coverage via `AppConfig::parse` tests in `src/config.rs` (18 tests including `minimal_config_loads_with_defaults` at L264 which covers the exact TOML shape `save_config` emits). CRITIC narrowed the finding to "lower severity" and ultimately conceded it is not proportionate to block given the pre-existing integration test infrastructure gap. **Resolved: not blocking.**
- **CSV silent truncation** — CRITIC argued silent truncation is a functional defect for non-technical users; ADVOCATE cited the fix manifest's explicit choice of option (a) ("a truncated CSV is less harmful than a corrupted one") as the governing exit criterion. CRITIC conceded this was a new requirement beyond the prior mandate, not a defect against it. **Resolved: not blocking, recommended as follow-up.**
- **CORS prefix matching** — CRITIC framed it as a manifest-prescribed fix left unaddressed; ARBITER challenged this citing `.council/fix-manifest.md:29` which classifies it as "informational (not blocking), low severity for localhost tool." CRITIC conceded immediately. **Resolved: not blocking.**

### Concessions
- **CRITIC** conceded Gap 3 (CORS) to **ARBITER** — fix manifest explicitly classified as non-blocking informational
- **CRITIC** conceded Gap 1 (save_config test coverage) to **ADVOCATE** — indirect coverage is substantial, integration test gap is pre-existing infrastructure constraint
- **CRITIC** conceded Gap 2 (CSV truncation) to **ADVOCATE** — prior council explicitly chose this tradeoff and defined the exit criterion; new requirement was outside the mandate
- **ADVOCATE** conceded that from a UX perspective, silent CSV truncation is "a genuine problem" for non-technical users — but classified it as a follow-up, not a blocker

### Regression Lineage
No regression lineage — no prior fix involvement. The two blocking findings from the prior council (`fix-manifest.md:10-16` u32 overflow, `fix-manifest.md:18-26` CSV sentinel) are both verified fixed.

### Arbiter Recommendation
**FOR**

The branch meets all exit criteria defined by the prior council's fix manifest. Both blocking findings are verified fixed with correct implementations. CRITIC raised three gaps, all of which were either conceded or resolved through debate. The branch maintains the SQL injection prevention invariant documented in CLAUDE.md, enforces table access control on every table-scoped endpoint, hardens the setup wizard against injection and blind writes, and includes substantive test coverage (34+ tests across config, API, setup, and DB modules). No unresolved blocking findings remain.

### Conditions (if CONDITIONAL)
None — recommendation is FOR without conditions.

### Suggested Fixes
No issues identified that require in-PR fixes. All prior blocking findings are already fixed.

### Follow-Up Recommendations (non-blocking)
These items were raised during the debate and acknowledged by both sides as real but non-blocking concerns:

1. **CSV truncation UX** — Track as a follow-up issue: add a client-detectable signal for truncated CSV exports (e.g., trailing comment row in valid CSV format, or a non-streaming fallback endpoint with explicit error). Acknowledged by ADVOCATE as "a genuine UX problem" for non-technical users.
2. **save_config integration test** — When live-DB test infrastructure is established for the project, add an end-to-end test for the `save_config` success path at `src/api/setup.rs:187`.
3. **CORS exact matching** — Per fix manifest informational note: tighten `src/main.rs:47` from `starts_with("http://localhost")` to `s == "http://localhost" || s.starts_with("http://localhost:")`. Low severity, zero-cost fix.
