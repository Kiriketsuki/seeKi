# SeeKi Manual QA Checklist

## How to Run

### Prerequisites
- PostgreSQL database running and accessible
- `seeki.toml` configured in project root or `~/.config/seeki/config.toml`
- Rust toolchain installed (1.85+)
- Modern browser (Chrome/Chromium recommended for best glassmorphism support)

### Starting the Application
```bash
# Option 1: Using cargo
cargo run

# Option 2: Using just
just dev

# App will be available at http://127.0.0.1:3141
```

### Resetting State for Testing
```bash
# Clear browser localStorage
# Chrome DevTools → Application → Local Storage → http://127.0.0.1:3141 → Clear All

# Force setup wizard to appear
rm seeki.toml  # or ~/.config/seeki/config.toml

# Clear persisted settings
# In browser console:
localStorage.clear()
location.reload()
```

---

## 1. Visual Fidelity

| Check | Expected Result | Status | Notes |
|-------|----------------|--------|-------|
| Sidebar glassmorphism | Frosted glass effect with backdrop blur on sidebar background | [ ] | Should use `--sk-glass-sidebar` token |
| Toolbar glassmorphism | Frosted glass effect on toolbar at top of grid | [ ] | Should use CSS backdrop-filter |
| Status bar glassmorphism | Frosted glass effect on bottom status bar | [ ] | Should use `--sk-glass-statusbar` token |
| CSS design tokens applied | All UI elements use `--sk-*` custom properties for colors, spacing, borders | [ ] | Check DevTools computed styles |
| Color badges render | Specific column values display with colored badges (configurable per table) | [ ] | E.g., status="active" → green badge |
| NULL cell hatching | Cells with NULL values show diagonal hatched pattern background | [ ] | Pattern should be subtle, not overwhelming |
| NULL cell italic text | NULL cells display italic "NULL" text | [ ] | Text should be muted color |
| Timestamp formatting | ISO timestamps display as "MMM D, H:MM AM/PM" | [ ] | E.g., "2024-01-15T14:30:00Z" → "Jan 15, 2:30 PM" |
| Boolean formatting | Boolean values display as "Yes"/"No" badges | [ ] | Green for true, red/gray for false |
| Number alignment | Numeric columns are right-aligned | [ ] | Check integers, floats, decimals |
| Inter font for UI | UI labels, buttons, sidebar use Inter variable font | [ ] | Check font-family in DevTools |
| JetBrains Mono for data | Grid cells and code-like content use JetBrains Mono variable | [ ] | Check grid cell font-family |
| Border consistency | Borders use `--sk-border` token consistently | [ ] | Check sidebar, toolbar, grid |
| Spacing consistency | Margins and padding use `--sk-space-*` tokens | [ ] | Check sidebar items, toolbar buttons |
| Responsive at 1920px | UI renders correctly at desktop resolution | [ ] | Check all elements visible, no overflow |
| Responsive at 1366px | UI adjusts for laptop resolution | [ ] | Sidebar may need to be narrower |
| Responsive at 1024px | UI works at tablet landscape | [ ] | Consider sidebar auto-collapse |
| Glass blur renders | Backdrop-filter blur effect visible in supported browsers | [ ] | Chrome/Edge should work; Firefox may not |

---

## 2. Accessibility

| Check | Expected Result | Status | Notes |
|-------|----------------|--------|-------|
| Tab navigation - toolbar | Tab key navigates through search, filter, column, export buttons | [ ] | Should have visible focus ring |
| Tab navigation - sidebar | Tab key navigates through table list items | [ ] | Should skip collapsed items |
| Tab navigation - grid | Tab enters grid, arrow keys navigate cells | [ ] | RevoGrid default behavior |
| Enter/Space on buttons | Toolbar buttons activate with keyboard | [ ] | Test search toggle, filter toggle, export |
| ARIA labels on toolbar | All icon-only buttons have aria-label | [ ] | E.g., aria-label="Toggle search" |
| ARIA labels on wizard | Wizard steps have proper labels and roles | [ ] | role="tablist", aria-current, etc. |
| Sort state announced | Column sort state has aria-sort attribute | [ ] | "ascending", "descending", "none" |
| Focus on search open | Search input auto-focuses when toggled on | [ ] | Test with icon click and Ctrl+K |
| Focus management - wizard | Focus moves to next step header on Next | [ ] | Assistive tech should announce step |
| Color contrast - text | All text meets WCAG AA (4.5:1 for normal, 3:1 for large) | [ ] | Use browser contrast checker |
| Color contrast - badges | Badge text on colored backgrounds meets contrast | [ ] | Check Yes/No badges, status badges |
| No keyboard traps | Tab/Shift+Tab can always exit any component | [ ] | Test filter dropdowns, column menu |
| Esc key behavior | Esc closes search bar, filter dropdown, column menu | [ ] | Should not close wizard |

---

## 3. Setup Wizard

| Check | Expected Result | Status | Notes |
|-------|----------------|--------|-------|
| Wizard renders on first run | App shows 4-step wizard when no config exists | [ ] | Delete seeki.toml to test |
| Progress dots display | 4 dots at top showing current step highlighted | [ ] | Steps: Connect, Tables, Branding, Confirm |
| Step 1 - URL mode default | "Database URL" radio selected by default | [ ] | Single textarea for full URL |
| Step 1 - URL mode validation | URL input accepts valid PostgreSQL URLs | [ ] | E.g., `postgresql://user:pass@host:5432/db` |
| Step 1 - Fields mode toggle | Clicking "Fields" radio shows individual input fields | [ ] | Host, Port, Database, User, Password |
| Step 1 - SSH toggle shows fields | "Connect via SSH tunnel" checkbox reveals SSH section | [ ] | SSH host, port, user, auth method |
| Step 1 - SSH key auth | SSH key path field appears when "SSH Key" selected | [ ] | Should accept path like ~/.ssh/id_rsa |
| Step 1 - SSH password auth | SSH password field appears when "Password" selected | [ ] | Input type="password" |
| Step 1 - SSH agent auth | No extra fields when "Agent" selected | [ ] | Will use ssh-agent |
| Step 1 - Test valid connection | "Test Connection" succeeds with valid credentials | [ ] | Shows green checkmark or success message |
| Step 1 - Test invalid connection | "Test Connection" fails gracefully with bad credentials | [ ] | Shows user-friendly error (no stack trace) |
| Step 1 - Next enabled after test | "Next" button enabled only after successful test | [ ] | Button should be disabled initially |
| Step 2 - Table list loads | Tables from connected DB display with checkboxes | [ ] | Should show schema.table or just table name |
| Step 2 - Select All works | "Select All" checks all table checkboxes | [ ] | All checkboxes should be checked |
| Step 2 - Deselect All works | "Deselect All" unchecks all checkboxes | [ ] | All checkboxes should be unchecked |
| Step 2 - Individual toggle | Clicking a table checkbox toggles selection | [ ] | Check multiple tables individually |
| Step 2 - Next enabled with selection | "Next" button enabled when at least 1 table selected | [ ] | Disabled when 0 tables selected |
| Step 3 - Branding preview | Logo upload and custom title show live preview | [ ] | Preview pane should update immediately |
| Step 3 - Logo file upload | File input accepts image files (PNG, JPG, SVG) | [ ] | Should reject non-image files |
| Step 3 - Custom title input | Text input allows custom app title | [ ] | Default "SeeKi" if empty |
| Step 4 - Summary displays | Final step shows summary of choices | [ ] | Connection string (sanitized), table count, branding |
| Step 4 - Save triggers reload | "Save & Launch" creates config and loads normal mode | [ ] | Should not require manual reload |
| Wizard step animation | Steps slide in/out when clicking Next/Back | [ ] | Smooth CSS transition |
| Back button works | "Back" navigates to previous step | [ ] | Except on step 1 (no back) |
| Wizard cannot skip steps | Cannot jump to step 3 without completing steps 1-2 | [ ] | Test direct URL manipulation if applicable |

---

## 4. Data Grid Interactions

| Check | Expected Result | Status | Notes |
|-------|----------------|--------|-------|
| Grid loads data | Rows from selected table display in grid | [ ] | Check first 50 rows load |
| Column headers display | All column names appear as headers | [ ] | Should use display names if configured |
| Sort ascending | Click column header → data sorts A-Z (or 0-9) | [ ] | Check visual sort indicator |
| Sort descending | Click header again → data sorts Z-A (or 9-0) | [ ] | Arrow icon should flip |
| Sort clear | Click header third time → returns to unsorted | [ ] | Original order restored |
| Sort indicator visible | Sorted column shows arrow icon (↑ or ↓) | [ ] | Unsorted columns show no icon |
| Filter input per column | Each column header has filter input field | [ ] | When filter mode enabled |
| Filter debounce | Filter typing waits ~300ms before applying | [ ] | Prevents excessive requests |
| Filter case-insensitive | Filtering "hello" matches "Hello", "HELLO" | [ ] | Test with string columns |
| Multiple filters AND together | Filter col1="A" AND col2="B" shows only matching rows | [ ] | Apply filters to 2+ columns |
| Filter clear | Clearing filter input restores all rows | [ ] | Backspace to empty |
| Global search | Toolbar search filters across all visible columns | [ ] | Should highlight matches |
| Pagination controls | First, Prev, Next, Last buttons navigate pages | [ ] | Check button enable/disable state |
| Pagination page input | Typing page number + Enter jumps to that page | [ ] | E.g., type "5" → go to page 5 |
| Pagination page size | Dropdown changes rows per page (25, 50, 100) | [ ] | Grid should reload with new size |
| Large dataset (1000+ rows) | Grid handles 1000+ rows with smooth scrolling | [ ] | Test pagination performance |
| Large dataset sort | Sorting 1000+ rows completes in <2s | [ ] | May require backend sort |
| Large dataset filter | Filtering 1000+ rows returns results quickly | [ ] | Backend should handle |
| Cell value accuracy | Cell values match database exactly | [ ] | Spot-check random rows with SQL query |
| Cell formatting consistency | All timestamp/boolean/number cells formatted per spec | [ ] | Check 20+ cells across types |
| Horizontal scroll | Grid scrolls horizontally for wide tables | [ ] | Test table with 20+ columns |
| Vertical scroll | Grid scrolls vertically for many rows | [ ] | Smooth with virtual scrolling |
| Sticky headers | Column headers remain visible during vertical scroll | [ ] | RevoGrid should handle this |

---

## 5. Toolbar

| Check | Expected Result | Status | Notes |
|-------|----------------|--------|-------|
| Toolbar visible | Toolbar appears at top of data grid area | [ ] | Below table title, above grid |
| Search icon click | Clicking search icon toggles search bar visibility | [ ] | Should slide in/out |
| Ctrl+K shortcut | Pressing Ctrl+K toggles search bar | [ ] | Works when grid has focus |
| Search bar auto-focus | Search input auto-focuses when opened | [ ] | Cursor should be blinking in input |
| Search bar global filter | Typing filters rows across all columns | [ ] | Case-insensitive match |
| Search bar closes | Clicking icon again or pressing Esc closes search bar | [ ] | Animation should reverse |
| Filter toggle icon | Filter icon shows enabled/disabled state | [ ] | Different color or icon when active |
| Filter toggle action | Clicking filter icon shows/hides per-column filter inputs | [ ] | Below each column header |
| Filter badge count | Small badge shows count of active filters | [ ] | E.g., "2" when 2 columns filtered |
| Filter badge hidden when 0 | No badge when no filters active | [ ] | Badge should disappear |
| Column visibility dropdown | Clicking column icon opens dropdown menu | [ ] | List of all columns with checkboxes |
| Column hide/show | Unchecking column immediately hides it from grid | [ ] | Re-checking shows it again |
| Column menu - Show All | "Show All" button checks all column checkboxes | [ ] | All columns become visible |
| Column persistence | Hidden columns stay hidden after page reload | [ ] | Uses localStorage |
| CSV export button | Clicking export icon triggers CSV download | [ ] | File should be `{table_name}.csv` |
| CSV export filename | Downloaded file named correctly | [ ] | E.g., "users.csv" for users table |
| CSV export content | CSV contains visible rows with correct values | [ ] | Open in spreadsheet to verify |
| CSV export headers | First row contains column names | [ ] | Match visible column display names |
| CSV export filtered data | Export respects active filters/search | [ ] | Only filtered rows exported |
| Toolbar responsive | Toolbar buttons stack or shrink gracefully on narrow screens | [ ] | Test at 1024px width |

---

## 6. Navigation

| Check | Expected Result | Status | Notes |
|-------|----------------|--------|-------|
| Sidebar renders | Left sidebar shows list of tables | [ ] | Visible on app load |
| Table display names | Tables show configured display names, not raw names | [ ] | Check config branding section |
| Table list scrollable | Sidebar scrolls if table list is long | [ ] | Test with 20+ tables |
| Table click switches view | Clicking a table loads its data in grid | [ ] | Grid should refresh |
| Active table highlighted | Currently selected table has different background/border | [ ] | Visual indicator of selection |
| Sidebar search input | Search box at top of sidebar filters table list | [ ] | Type to filter |
| Sidebar search filtering | Typing "user" shows only tables with "user" in name | [ ] | Case-insensitive |
| Sidebar search clear | Clearing search shows all tables again | [ ] | Backspace to empty |
| Sidebar collapse button | Button to collapse sidebar to icon-only mode | [ ] | Arrow or hamburger icon |
| Sidebar collapse animation | Sidebar smoothly animates width change | [ ] | CSS transition |
| Sidebar expand | Clicking collapsed sidebar or toggle button expands it | [ ] | Reverse animation |
| Sidebar state persistence | Collapsed state persists after page reload | [ ] | Uses localStorage |
| Table switch preserves filters | Switching tables clears filters (or preserves if applicable) | [ ] | Decide expected behavior |
| Table switch resets sort | Switching tables resets to unsorted state | [ ] | Or preserves column config if shared |
| Browser back/forward | Browser back button after table switch returns to previous table | [ ] | History API integration |

---

## 7. SSH Tunnel Scenarios

| Check | Expected Result | Status | Notes |
|-------|----------------|--------|-------|
| SSH key auth - valid | Connection succeeds with correct SSH key path | [ ] | Use ~/.ssh/id_rsa or id_ed25519 |
| SSH key auth - invalid key | Connection fails gracefully with wrong key | [ ] | User-friendly error message |
| SSH key auth - missing key | Connection fails if key file doesn't exist | [ ] | Error: "SSH key not found" |
| SSH password auth - valid | Connection succeeds with correct SSH password | [ ] | Password field should be masked |
| SSH password auth - wrong password | Connection fails with incorrect SSH password | [ ] | Error should not expose password |
| SSH agent auth - valid | Connection succeeds when ssh-agent has key loaded | [ ] | Run `ssh-add` first |
| SSH agent auth - no agent | Connection fails if ssh-agent not running | [ ] | Error: "Could not connect to agent" |
| SSH + DB success | SSH tunnel connects AND database connection succeeds | [ ] | End-to-end happy path |
| SSH success + DB fail | SSH tunnel connects but DB credentials invalid | [ ] | Error should indicate DB auth failure |
| SSH timeout | Connection fails gracefully if SSH host unreachable | [ ] | Timeout after ~10s |
| SSH wrong host | Connection fails with unknown host error | [ ] | E.g., "Connection refused" |
| SSH wrong port | Connection fails if SSH port incorrect | [ ] | Common mistake: 22 vs 2222 |
| SSH custom port | Connection succeeds with non-standard SSH port | [ ] | Test with port 2222 or similar |

---

## 8. Edge Cases & Exploratory Testing

| Check | Expected Result | Status | Notes |
|-------|----------------|--------|-------|
| Very long text value (1000+ chars) | Cell displays truncated text with ellipsis | [ ] | Hover or click should show full value |
| Unicode characters | Unicode text (中文, العربية, emoji 🎉) renders correctly | [ ] | No mojibake or boxes |
| Emoji in column names | Columns with emoji in name display properly | [ ] | E.g., "status_🔥" |
| Emoji in data values | Emoji in cell values render correctly | [ ] | Including in badges if applicable |
| Special chars in column names | Columns like "user-name" or "email@domain" load | [ ] | Should be sanitized in SQL but displayed raw |
| Empty table (0 rows) | Grid shows "No data" or empty state message | [ ] | Not a blank screen |
| Table with 1 row | Grid renders correctly with single row | [ ] | Pagination should show 1 of 1 |
| Table with 1 column | Grid renders single-column layout | [ ] | Should not break layout |
| Table with 50+ columns | Wide table scrolls horizontally | [ ] | Test usability |
| Column name with spaces | "First Name" column displays correctly | [ ] | Headers should wrap or truncate |
| Column name collision | Tables with same column names (e.g., "id") don't conflict | [ ] | Scoped per table |
| Rapid table switching | Clicking 5 different tables in quick succession | [ ] | No race conditions, last click wins |
| Table switch during load | Switching tables while previous table still loading | [ ] | Should cancel previous request |
| Network interruption | Disconnect network mid-data load | [ ] | Error message, option to retry |
| Slow database response | DB takes 5+ seconds to return rows | [ ] | Loading spinner, no timeout under 30s |
| Browser back after wizard | Back button after completing wizard returns to wizard | [ ] | Or disabled if config saved |
| Browser forward | Forward button works after going back | [ ] | History should be intact |
| Multiple tabs open | Opening app in 2+ tabs simultaneously | [ ] | Each tab should work independently |
| Multiple tabs - state sync | Changes in tab A don't affect tab B | [ ] | Or decide if state should sync |
| Page reload during data load | Refresh browser mid-load | [ ] | Should restart cleanly |
| LocalStorage quota exceeded | App handles localStorage full error | [ ] | Unlikely but graceful degradation |
| Corrupt localStorage data | App recovers from invalid JSON in localStorage | [ ] | Should clear and reset |

---

## 9. UX Polish

| Check | Expected Result | Status | Notes |
|-------|----------------|--------|-------|
| Sidebar collapse smooth | Transition duration ~200-300ms, eased | [ ] | Not instant or janky |
| Search bar toggle smooth | Search bar slides in/out smoothly | [ ] | Same timing as sidebar |
| Wizard step transition | Steps fade/slide in smoothly | [ ] | No flicker or jump |
| Button hover states | Toolbar buttons show hover effect (scale, color change) | [ ] | Subtle, not jarring |
| Table row hover | Grid rows highlight on mouse hover | [ ] | Light background change |
| Loading spinner | Data loading shows spinner or skeleton | [ ] | Centered in grid area |
| Loading state - table switch | Brief loading indicator when switching tables | [ ] | User knows action is processing |
| Error message - connection fail | "Could not connect to database" (not raw error) | [ ] | No SQL or stack traces |
| Error message - query fail | "Error loading data" with retry button | [ ] | Friendly, actionable |
| Error message - SSH fail | "SSH tunnel connection failed: [reason]" | [ ] | Avoid technical jargon |
| Success feedback - wizard | Green checkmark or toast on successful save | [ ] | Confirms action completed |
| Success feedback - export | "CSV exported successfully" message | [ ] | Brief toast notification |
| Tooltip on icon buttons | Toolbar icons show tooltip on hover | [ ] | E.g., "Toggle search (Ctrl+K)" |
| Focus ring visibility | Focused elements have clear visible ring | [ ] | Meets WCAG focus requirements |
| Animation frame rate | All animations run at 60fps, no stuttering | [ ] | Open DevTools performance tab |
| No layout shift | Loading data doesn't cause page elements to jump | [ ] | Reserve space for grid |
| No flash of unstyled content | CSS loads before render (no FOUC) | [ ] | Especially fonts |
| Favicon displays | Browser tab shows SeeKi icon | [ ] | Check favicon.ico or SVG |
| Page title updates | Browser tab title shows current table name | [ ] | E.g., "SeeKi - users" |

---

## Sign-off

| Tester | Date | Build Version | Pass/Fail | Notes |
|--------|------|---------------|-----------|-------|
|        |      |               |           |       |

**Pass Criteria**: All items marked `[x]` with no critical failures. Minor issues may be documented in Notes column for future fix.

