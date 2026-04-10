# Fix Manifest — PR #30: epic: Toolbar, Column Management & Export

Council verdict: CONDITIONAL | 2026-04-10 | 8 findings (8 verified) | All fixed (3 sessions)

---

## Session 1 Fixes (previously applied)

### 1. ARIA mismatch — columns dropdown missing role="dialog"
- **File**: `frontend/src/components/ColumnDropdown.svelte`
- **Line**: 30
- **Type**: a11y / **Severity**: high
- **Fix**: Added `role="dialog"` and `aria-label="Column visibility"` to `.dropdown` div
- **Citations**: `Toolbar.svelte:128` (aria-haspopup="dialog"), `ColumnDropdown.svelte:30`

### 2. Double horizontal padding on TableHeader
- **File**: `frontend/src/components/TableHeader.svelte`
- **Line**: 24
- **Type**: layout / **Severity**: high
- **Fix**: Changed `.table-header` `padding: 0 var(--sk-space-2xl)` → `padding: 0`
- **Citations**: `TableHeader.svelte:24`, `App.svelte:576`

### 3. Export button always enabled with no table selected
- **File**: `frontend/src/components/Toolbar.svelte`
- **Line**: 141
- **Type**: ux / **Severity**: medium
- **Fix**: Added `hasTable?: boolean` prop; `disabled={!hasTable}` on export button
- **Citations**: `Toolbar.svelte:141`, `App.svelte:486`

### 4. Ctrl+K blocked when search input focused
- **File**: `frontend/src/App.svelte`
- **Line**: 110
- **Type**: ux / **Severity**: medium
- **Fix**: Added `isSearchField` guard so Ctrl+K works when focus is in the search input
- **Citations**: `App.svelte:110-111`

### 5. "Show All" always enabled when all columns visible
- **File**: `frontend/src/components/ColumnDropdown.svelte`
- **Line**: 39
- **Type**: ux / **Severity**: low
- **Fix**: Added `disabled={hiddenCount === 0}` to Show All button
- **Citations**: `ColumnDropdown.svelte:39`

---

## Session 3 Fixes (this session)

### 6. Static aria-label on filter/columns buttons hides dynamic badge count from AT
- **File**: `frontend/src/components/Toolbar.svelte`
- **Lines**: 103, 131
- **Type**: a11y / **Severity**: medium
- **Fix**: `aria-label="Toggle filters"` → `aria-label={filterTitle}`; `aria-label="Manage columns"` → `aria-label={columnsTitle}`. Both derived values already existed in the component.
- **Citations**: `Toolbar.svelte:103`, `Toolbar.svelte:131`, `Toolbar.svelte:60-69`

### 7. type="search" + custom X button renders two clear controls on Chrome/Safari
- **File**: `frontend/src/App.svelte`
- **Line**: 505
- **Type**: ux / **Severity**: medium
- **Fix**: Changed `type="search"` → `type="text"`; custom `<button class="clear-search">` is now the sole clear mechanism
- **Citations**: `App.svelte:504`, `App.svelte:512-519`

### 8. Export button disabled title gives no context
- **File**: `frontend/src/components/Toolbar.svelte`
- **Line**: 147
- **Type**: ux / **Severity**: low
- **Fix**: `title="Export CSV"` → `title={hasTable ? 'Export CSV' : 'Select a table to export'}`; `aria-label` likewise derived
- **Citations**: `Toolbar.svelte:143-150`

---

## Conditions
All 8 findings fixed. `svelte-check` passes: 0 errors, 0 warnings.

## Test Command
```
cd frontend && npx svelte-check
```

## Raw Data
council-result.json: .council/council-result.json
