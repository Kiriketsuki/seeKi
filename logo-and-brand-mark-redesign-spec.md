# Feature: Logo and Brand Mark Redesign

## Context

Issue #53. The current favicon is a placeholder purple lightning-bolt (`#863bff`) with 16 gaussian-blur filter defs — marketing-tier noise at favicon scale, and no clear brand connection to "SeeKi" or the product's read-only data-viewer identity. The UI's `--sk-accent` token is currently teal `#00A9A5`, disconnected from the favicon. This feature unifies the mark, wordmark, and colour tokens into one deliberate brand system.

---

## Overview

**User Story**: As a SeeKi user, I want a logo that signals "seeing into data" (not ERDs, not SQL, not enterprise) so that the product's identity matches its read-only, spreadsheet-first philosophy.

**Problem**: The current favicon is a placeholder with no brand coherence — it's visually noisy (16 filter defs, many unused at 32px), thematically unrelated to the product, and disconnected from the UI's colour tokens. Nothing else in the product reinforces the "SeeKi" name.

**Out of Scope**:
- Animated logos
- Dark-mode-specific asset variants (handled via CSS `prefers-color-scheme` on the same SVG)
- Marketing illustration system
- `logo-hero.svg` marketing variant (deferred — YAGNI for v0.1)

---

## Success Condition

> This feature is complete when the new SK-ligature mark ships as the favicon and sidebar logo, the amber spot colour replaces `--sk-accent` across the UI, and all deliverable files (favicon SVG/ICO, apple-touch-icon, mark, wordmark, mono) live in `frontend/public/` and pass the 16×16 silhouette, ≤4KB, 3:1 contrast, and no-filter tests.

---

## Design Decisions (from brainstorm)

**Mark** — V3a ligature:
- Single stroke: S top curl loops into a shared vertical stem
- Stem drops to the junction (visual centre); stem's lower half bends left to close the S bottom
- K's upper and lower chevrons branch from the junction
- Amber aperture dot sits inside the S's top curl — the "eye that sees into data"

**Palette**:
- `#111111` ink (mark on light / background on dark)
- `#f4f4f2` cream (background on light / mark on dark)
- `#ff9500` amber (light) / `#ffb000` amber (dark) — aperture dot only

**Wordmark**:
- **JetBrains Mono** (already in the stack — `@fontsource-variable/jetbrains-mono`)
- "SeeKi" camel-case, baseline-aligned to mark, mark height = cap height
- Wordmark rendered in ink/cream only; amber never appears in text

**Geometry**:
- 48px base viewbox, 4-unit grid alignment
- Corner radius 2px at 48px (~4.2%)
- Stroke ~4px at 48px, scales proportionally
- No gradients, no filters in mark/favicon variants

---

## Open Questions

| # | Question | Resolved |
|:--|:---------|:---------|
| 1 | Confirm `logo-hero.svg` can be deferred to v0.2 (spec explicitly dropped it) | [x] |
| 2 | Does the wordmark need a tagline variant ("SeeKi — see into your data")? | [ ] |

---

## Scope

### Must-Have
- **Mark SVG**: V3a ligature rendered cleanly, ≤4KB, no filters, grid-aligned
- **Favicon set**: `favicon.svg` + `favicon.ico` (16/32/48) + `apple-touch-icon.png` (180×180)
- **Logo variants**: `logo-mark.svg`, `logo-wordmark.svg`, `logo-mono.svg`
- **Token update**: `--sk-accent` → amber; add `--sk-brand-primary` / `--sk-brand-surface` tokens if not already present
- **index.html link tags**: updated to reference new files including `apple-touch-icon.png`
- **Sidebar / app chrome**: any hard-coded old favicon reference replaced with `logo-mark.svg`
- **Contrast + silhouette tests**: mark passes 16×16 silhouette legibility and 3:1 contrast on both backgrounds

### Should-Have
- **README update**: swap the old purple-bolt wordmark for the new horizontal wordmark
- **Wordmark in setup wizard branding step**: `SetupStep3Branding.svelte` currently uses brand tokens — render the new wordmark there

### Nice-to-Have
- **SVG sprite optimisation** (single sprite sheet if multiple marks are inlined)
- **Storybook / component gallery entry** for the logo system

---

## Technical Plan

**Affected Components**:
- `frontend/public/favicon.svg` — replace
- `frontend/public/favicon.ico` — new
- `frontend/public/apple-touch-icon.png` — new
- `frontend/public/logo-mark.svg` — new
- `frontend/public/logo-wordmark.svg` — new
- `frontend/public/logo-mono.svg` — new
- `frontend/index.html` — update `<link>` tags (add apple-touch-icon, keep favicon)
- `frontend/src/theme/tokens.css` — update `--sk-accent`, add `--sk-brand-primary`, `--sk-brand-surface`
- `README.md` — swap logo reference
- Any Svelte component that inlines the old mark (sidebar branding, setup wizard step 3)

**Token Changes**:
```css
:root {
  --sk-brand-primary: #111111;
  --sk-brand-surface: #f4f4f2;
  --sk-accent: #ff9500;         /* was #00A9A5 */
}
:root[data-theme="dark"] {
  --sk-brand-primary: #f4f4f2;
  --sk-brand-surface: #0e1116;
  --sk-accent: #ffb000;
}
```

**Dependencies**: None added. JetBrains Mono already loaded.

**Risks**:
| Risk | Likelihood | Mitigation |
|:-----|:-----------|:-----------|
| Amber on cream has lower contrast than expected | Med | Contrast check (3:1 min) during asset generation; darken amber to `#e68600` if fails |
| Existing `--sk-accent: #00A9A5` consumers expect teal (18 call sites) | High | Audit all 18 sites visually after token swap; amber is warmer — focus rings / active states should still read as "attention" |
| ICO generation requires a separate tool (imagemagick / svg-to-ico) | Low | Document the build step; commit the generated ICO directly |
| SK ligature at 16×16 loses the amber dot | Med | Test mark at 16px early; if dot disappears, enlarge it or drop it at favicon scale |

---

## Acceptance Scenarios

```gherkin
Feature: Logo and brand mark redesign

  Scenario: Favicon renders in the browser tab
    Given a fresh build of the frontend
    When I load the app in Chrome, Firefox, and Safari
    Then the browser tab shows the new SK-ligature mark
    And the mark is visually legible at 16×16

  Scenario: App chrome uses the new mark
    Given the app is running
    When I view the sidebar
    Then the SeeKi brand area shows the new logo-wordmark (mark + "SeeKi" in JetBrains Mono)
    And no references to the old purple lightning-bolt remain

  Scenario: Accent colour propagates to UI chrome
    Given the new tokens are committed
    When I open any component that uses --sk-accent (focus rings, active sort, setup wizard primary button)
    Then those elements render in amber, not teal

  Scenario: Mark survives monochrome
    Given the logo-mono.svg variant
    When rendered in a single colour (no amber dot)
    Then the SK silhouette still reads as intended (letterforms distinguishable)

  Scenario: Contrast passes WCAG 3:1 non-text minimum
    Given the mark rendered on #ffffff and on #0e1116
    When measured with a contrast tool
    Then both combinations meet or exceed 3:1

  Scenario: Favicon filesize budget
    Given the favicon.svg after authoring
    When its size is measured
    Then the file is ≤4KB and contains no <filter> or <feGaussianBlur> elements
```

---

## Task Breakdown

| ID   | Task                                                                   | Priority | Dependencies | Status  |
|:-----|:-----------------------------------------------------------------------|:---------|:-------------|:--------|
| T1   | Author the master SVG (V3a ligature, grid-aligned, amber dot)          | High     | None         | pending |
| T2   | Derive `favicon.svg` from master (strip wordmark, verify ≤4KB)         | High     | T1           | pending |
| T3   | Generate `favicon.ico` (16/32/48) from master                          | High     | T1           | pending |
| T4   | Generate `apple-touch-icon.png` 180×180                                | High     | T1           | pending |
| T5   | Derive `logo-mark.svg` (square, mark only, no filters)                 | High     | T1           | pending |
| T6   | Derive `logo-wordmark.svg` (mark + JetBrains Mono "SeeKi")             | High     | T1           | pending |
| T7   | Derive `logo-mono.svg` (no amber, single colour)                       | Med      | T1           | pending |
| T8   | Update `tokens.css`: `--sk-accent`, add brand-primary / brand-surface  | High     | None         | pending |
| T9   | Update `frontend/index.html` link tags (favicon + apple-touch-icon)    | High     | T2, T4       | pending |
| T10  | Replace sidebar / app-chrome logo references with new wordmark         | High     | T6           | pending |
| T11  | Visual audit — 18 `--sk-accent` call sites render correctly in amber   | Med      | T8           | pending |
| T12  | Contrast test: mark on #ffffff and #0e1116 meet 3:1                    | High     | T5           | pending |
| T13  | 16×16 silhouette test: favicon remains legible                         | High     | T2           | pending |
| T14  | Cross-browser check: Chrome, Firefox, Safari favicon renders           | High     | T2, T9       | pending |
| T15  | README logo swap                                                       | Med      | T6           | pending |
| T16  | Update `SetupStep3Branding.svelte` to render the new wordmark          | Med      | T6           | pending |

---

## Exit Criteria

- [ ] All Must-Have scenarios pass in CI (or manual verification where automation is absent)
- [ ] No regressions on `--sk-accent` consumers (18 sites audited visually)
- [ ] Favicon ≤4KB, no `<filter>` elements
- [ ] Mark passes 3:1 contrast on both light and dark
- [ ] 16×16 silhouette legibility confirmed
- [ ] Chrome + Firefox + Safari all render the new favicon
- [ ] Old purple bolt fully removed from repo (grep for `863bff` / `7e14ff` / `47bfff` returns zero hits)
- [ ] README logo updated

---

## References

- Issue: #53 — https://github.com/Kiriketsuki/seeKi/issues/53
- Branch: `task/53-task-redesign-logo-favicon-and-brand-mark`
- Brainstorm mockups: `.brainstorm/2396117-1776057869/content/` (concept-direction, sk-aperture, sk-ligature, sk-ligature-v2, sk-ligature-v3, palette, spot-colours)
- Related tokens: `frontend/src/theme/tokens.css:5`

---

*Authored by: Clault KiperS 4.6*
