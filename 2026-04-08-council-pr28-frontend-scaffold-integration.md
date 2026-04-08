---
## Adversarial Council — PR #28: Frontend Scaffold & Integration

> Convened: 2026-04-08 | Advocates: 1 | Critics: 2 | Rounds: 3/4 | Motion type: CODE

### Motion
PR #28: epic: Frontend Scaffold & Integration — Should this PR be merged as-is into main?

This PR scaffolds the Svelte 5 + Vite frontend for SeeKi, adds a glassmorphism theme system, component shells, mock API layer, TypeScript types, rust-embed for single-binary deployment, and a Justfile.

---

### Advocate Positions

**ADVOCATE-1**
- `src/embed.rs:13-29` — rust-embed integration is production-correct: exact-path serving, differentiated cache headers (`immutable` for hashed assets, `no-cache` for HTML), graceful 404 when dist is absent.
- `frontend/src/lib/types.ts:1-44` — TypeScript contract precisely mirrors the Rust API; no `any`; correct sequencing for future feature PRs.
- `frontend/src/lib/api.ts:41`, `api.ts:80` — `encodeURIComponent` on table names prevents URL injection; `VITE_MOCK` gate correctly disables explicit mock mode in production builds.
- All components use Svelte 5 runes (`$state`, `$props`, `$derived`, `$bindable`) correctly; `$derived` at `DataGrid.svelte:13-17` reactively transforms typed column data.
- Disabled toolbar/pagination stubs are intentional scaffold signals, not broken features.
- Justfile covers all three development lifecycle scenarios (dev, dev-mock, build).
- Recommended CONDITIONAL merge: fix `Sidebar.svelte:33` in-PR; file tracked issues for catch-block error-handling and font bundling.

---

### Critic Positions

**CRITIC-1**
- `frontend/src/lib/api.ts:28-34, 46, 84, 94` — catch-block mock fallback fires on initial page load via `App.svelte:18-28` (`onMount` calls `fetchTables()` and `fetchDisplayConfig()` unconditionally), not only on user interaction. A non-technical user sees Aurrigo vehicle rows from first paint with no error signal.
- `frontend/src/components/Sidebar.svelte:31-34` — `handleToggle` calls `onToggle()` first, which synchronously flips `collapsed` via Svelte 5's `$bindable` signal graph; `String(!collapsed)` then negates the already-flipped value, writing the previous state to localStorage. Persistence feature does not work. Fix is one character; belongs in this PR.
- `frontend/index.html:7-9` — Google Fonts CDN links contradict single-binary deployment philosophy (`CLAUDE.md`).
- `frontend/src/components/DataGrid.svelte:21` — RevoGrid `theme="compact"` uses RevoGrid's own CSS variable namespace (`--rv-*`); no bridge from `--sk-*` tokens. Conceded as scaffold follow-up after ADVOCATE response.
- Recommended AGAINST merging as-is; CONDITIONAL acceptable if all three in-scope bugs fixed in-PR.

**CRITIC-2**
- `frontend/src/lib/api.ts:34, 46, 84, 94` — catch-block references to mock functions are live code paths; Rollup eliminates only statically-dead `if (USE_MOCK)` branches. All 495 lines of `mock.ts`, including Aurrigo domain table and column names, ship in the production JS bundle.
- `frontend/src/lib/mock.ts:8-18` — Mock table names (`vehicles`, `vehicles_log`, `events`, `faults`, `flights`) are Aurrigo AutoConnect business domain entities. A generic tool's canonical mock data should use neutral names.
- `console.warn('Backend unavailable, using mock data', e)` reads as operational telemetry, not a "known bug" marker. Future contributors will copy the pattern without architectural signal that it is provisional.
- `frontend/src/App.svelte:16` — `let isSetup = false` typed as a plain constant signals a compile-time decision; `$state(false)` correctly signals runtime reactivity and costs nothing.
- App.svelte state concentration (6 variables) conceded as appropriate for current scale.
- Recommended AGAINST merging as-is; noted that ADVOCATE's final position (CONDITIONAL) is a de facto concession that AS-IS is insufficient.

---

### Questioner Findings

QUESTIONER did not send a report during the debate. The ARBITER independently verified the following claims by reading source:

| Claim | Agent | Arbiter Finding |
|---|---|---|
| Sidebar `onMount` pattern is "correct, side-effect-safe" | ADVOCATE-1 | **UNSUBSTANTIATED** — arithmetic in `handleToggle` is inverted; bug confirmed |
| `VITE_MOCK` "ensures mock can never bleed into production builds" | ADVOCATE-1 | **UNSUBSTANTIATED** — only covers explicit path at L27; catch-block at L32-34 is unconditional |
| App.svelte God-component "migration cost higher later" | CRITIC-2 | **WITHDRAWN** by CRITIC-2; speculative claim on a 6-variable component |
| Setup mode is "permanently dead from frontend's perspective" | CRITIC-2 | **OVERSTATED** — comment at `App.svelte:42` explicitly tags "Epic 5"; narrowed by CRITIC-2 to `const` vs `$state` typing |

---

### Key Conflicts

- **Mock fallback fires on initial load vs. requires user interaction** — ADVOCATE claimed defect is "inert" because toolbar buttons are `disabled`. CRITIC-1 traced `App.svelte:18-28` showing `onMount` calls `fetchTables()` unconditionally. ADVOCATE conceded in full: the defect fires from first paint. **Resolved: CRITIC-1 correct.**

- **`VITE_MOCK` gate covers production mock paths** — ADVOCATE argued the gate "ensures mock can never bleed into production." Both critics identified the catch-block at `api.ts:32-34` as a separate unconditional code path. ADVOCATE conceded. **Resolved: Critics correct.**

- **Mock.ts ships in production bundle** — ADVOCATE claimed mock is "never compiled into the released binary." CRITIC-2 traced the dependency graph: catch-block references are live paths that prevent Rollup tree-shaking of mock module. ADVOCATE conceded. **Resolved: CRITIC-2 correct.**

- **Sidebar localStorage arithmetic** — ADVOCATE initially described pattern as "correct." CRITIC-1 traced the `$bindable` propagation: `onToggle()` flips `collapsed` synchronously; `String(!collapsed)` then saves the pre-toggle state. ARBITER verified source at `Sidebar.svelte:8, 31-34` and `App.svelte:47-48`. ADVOCATE conceded in full. **Resolved: CRITIC-1 correct.**

- **RevoGrid theming gap** — CRITIC-1 argued `--sk-*` tokens are not bridged to RevoGrid's internal CSS namespace. ADVOCATE argued this is expected scaffold scope. CRITIC-1 conceded RevoGrid theming is appropriate scaffold follow-up. **Resolved: ADVOCATE correct on scope framing.**

- **App.svelte state concentration** — CRITIC-2 raised as God-component concern. ADVOCATE correctly responded that 6 state variables in a root orchestrator is appropriate for this component count. CRITIC-2 conceded. **Resolved: ADVOCATE correct.**

- **Sidebar fix timing** — ADVOCATE initially argued one-line fix can land in next feature PR. CRITIC-1 argued a bug identified in review on a zero-cost fix belongs in this PR. ADVOCATE conceded in full. **Resolved: CRITIC-1 correct.**

---

### Concessions

- **ADVOCATE-1** conceded: mock catch-block fires in production (medium severity, not "low"), mock.ts ships in production bundle (tree-shaking analysis correct), sidebar localStorage arithmetic is wrong, sidebar fix belongs in this PR, CDN fonts violate stated architecture, Aurrigo domain names are a coupling concern.
- **CRITIC-1** conceded: RevoGrid theming integration is appropriate scaffold follow-up; not a merge-blocking defect.
- **CRITIC-2** conceded: App.svelte state concentration (6 variables) is appropriate for current scale; RevoGrid theming not a blocker from CRITIC-2's position.

---

### Regression Lineage

No regression lineage — no prior fix involvement. This is an epic-opening scaffold PR.

---

### Arbiter Recommendation

**CONDITIONAL**

The structural contributions of this PR are sound: rust-embed integration is production-correct (`src/embed.rs:13-47`), the TypeScript API contract is established (`frontend/src/lib/types.ts`), Svelte 5 runes are used correctly throughout, the CSS token system is well-namespaced, and the Justfile covers all development workflows. The scaffold achieves its stated structural goals.

However, the debate confirmed four defects — all verified against source — that required in-PR remediation per the Honest Findings Protocol. Three of the four were conceded by ADVOCATE-1 before the debate closed. All four fixes have been applied to the PR files directly (see Fixes section). With those fixes applied, the PR is ready to merge.

The motion as stated ("merged as-is") fails: "as-is" contained confirmed broken behavior at first paint. With the applied fixes, the motion is FOR.

---

### Conditions (if CONDITIONAL)

The following fixes have been applied directly to PR files by the council process:

1. `frontend/src/components/Sidebar.svelte:31-34` — capture `nextCollapsed` before calling `onToggle()` to avoid reading the already-flipped binding value.
2. `frontend/src/lib/api.ts` — removed all four catch-block mock fallbacks; errors now propagate to callers. Side effect: `mock.ts` is now fully tree-shakeable in production builds (all remaining mock references are behind statically-eliminated `if (USE_MOCK)` branches).
3. `frontend/index.html` — removed Google Fonts CDN `<link>` tags; system font fallback activates via `tokens.css:47-48` (`system-ui, -apple-system, sans-serif`).
4. `frontend/src/App.svelte:16` — changed `let isSetup = false` to `let isSetup: boolean = $state(false)` to correctly signal runtime reactivity to future contributors.

---

### Suggested Fixes

#### Fixes (all in-PR — applied)

- **Sidebar localStorage inversion** — `frontend/src/components/Sidebar.svelte` L31-34 — MEDIUM — `handleToggle` wrote the pre-toggle state to localStorage because `onToggle()` propagates synchronously via `$bindable`, flipping `collapsed` before `String(!collapsed)` executes. Fixed by capturing `nextCollapsed = !collapsed` before `onToggle()`.
  CITE: `frontend/src/components/Sidebar.svelte` L31-34

- **Catch-block mock fallback in production** — `frontend/src/lib/api.ts` L28-34, L44-47, L82-85, L92-95 — HIGH — all four API functions caught any network/HTTP error and silently returned mock data with only `console.warn`. Fired on initial page load via `App.svelte:18-28` `onMount` before any user interaction. Non-technical users would see Aurrigo vehicle rows displayed as real data. Fixed by removing catch blocks; errors propagate to callers. As a consequence, `mock.ts` is now fully eliminated from production bundles by Rollup tree-shaking.
  CITE: `frontend/src/lib/api.ts` L28-34

- **CDN font dependency** — `frontend/index.html` L7-9 — LOW — Google Fonts `<link>` tags required internet access at runtime, contradicting the single-binary local-deployment design principle in `CLAUDE.md`. Graceful system-font fallback already defined in `tokens.css:47-48`. Removed; self-hosting Inter and JetBrains Mono is a follow-up task.
  CITE: `frontend/index.html` L7-9

- **Setup mode `const` vs `$state`** — `frontend/src/App.svelte` L16 — LOW — `let isSetup = false` typed as a plain variable signals a compile-time constant to future contributors; `$state(false)` correctly communicates that this is a reactive runtime value awaiting detection logic. Fixed in one word.
  CITE: `frontend/src/App.svelte` L16

#### PR Description Amendments

- Add note that `mock.ts` is now fully excluded from production bundles (consequence of catch-block removal).
- Add note that fonts fall back to system fonts pending a follow-up task to self-host Inter and JetBrains Mono.
- File a follow-up issue: replace Aurrigo-specific mock table names in `mock.ts:8-18` with generic domain-neutral names (`users`, `orders`, `products`, `events`, `logs`) before any public release or demo.
- File a follow-up issue: add error state to `App.svelte` `onMount` so API failures surface a visible error message rather than leaving the table list empty with no explanation.

#### Critical Discoveries (informational)

None. No findings met the Critical Discovery threshold (OWASP Top 10, data loss, or compliance blocker).

---

*Council closed: 2026-04-08 | Arbiter: ARBITER (claude-sonnet-4-6)*
