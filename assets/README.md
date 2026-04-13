# SeeKi Docs — Shared Assets

Shared visual foundation for the SeeKi documentation site. All pages under `/docs` consume
`assets/css/site.css`, the self-hosted fonts under `assets/fonts/`, and the logo assets
under `assets/img/`.

## Font choice

**UI (body + headings): [Hanken Grotesk](https://fonts.google.com/specimen/Hanken+Grotesk)**
— humanist grotesque, open license (SIL OFL), served here as a variable woff2 so every weight
between 100 and 900 resolves to a single file. Picked because it's warm and slightly
mechanical without being Inter: the x-height and aperture read as a well-kept ledger rather
than as a designer default. SeeKi's tone is "quiet confidence," and Hanken carries that at
body sizes while still feeling neutral at display sizes.

**Tabular / monospace: [Commit Mono](https://commitmono.com/)** 400 + 700 — humanist mono,
also SIL OFL. Used for code blocks, inline code, and anywhere the docs reproduce SeeKi's
grid feel. It has the same data-entry character SeeKi wants in the product grid without the
JetBrains Mono reflex. Downloaded as OTF from the GitHub release and converted to woff2
locally with `woff2_compress`; the sources live in `fonts/commit-mono-400.woff2` and
`commit-mono-700.woff2`.

Neither Inter nor JetBrains Mono appears anywhere in this site — both are on the
`.impeccable.md` reflex-reject list.

## Class naming conventions

All classes and CSS variables are prefixed `sk-` to avoid clashes with future additions
(markdown renderer output, embedded widgets, etc.). Structure follows a light BEM:
`block__element--modifier`.

### Layout

| Class | Purpose |
|---|---|
| `.sk-shell` | Page-level grid: topbar + sidebar + content |
| `.sk-topbar` | Translucent top chrome (brand + theme toggle + search) |
| `.sk-sidebar` | Translucent left nav |
| `.sk-sidebar__section`, `.sk-sidebar__section-title` | Nav grouping |
| `.sk-sidebar__link`, `.sk-sidebar__link--active` | Nav items |
| `.sk-content` | Main scroll column |
| `.sk-prose` | Typographic container for markdown (max-width 72ch) |
| `.sk-breadcrumb`, `.sk-breadcrumb__sep` | Breadcrumb |
| `.sk-toc`, `.sk-toc__title` | On-page table of contents |
| `.sk-brand`, `.sk-brand__mark`, `.sk-brand__word` | Brand lockup |
| `.sk-theme-toggle` | Light/dark switch |

### Content primitives (inside `.sk-prose`)

| Class | Purpose |
|---|---|
| `.sk-callout`, `.sk-callout--note\|tip\|warn\|danger` | Notices |
| `.sk-code` (on `<pre>`), `code.sk-code--inline` | Code blocks + inline |
| `.sk-kbd` | Keyboard key indicator |
| `.sk-diagram` | Figure container for SVG/PNG diagrams |
| `.sk-table` | Doc tables (distinct from the product grid) |
| `.sk-badge--yes\|no\|null` | Yes/No/NULL examples referencing product |
| `.sk-anchor` | Heading anchor link |

### Tokens

Every colour, font, spacing, and radius is a `--sk-*` custom property declared at `:root`.
Dark theme is a genuine counterpart applied via `[data-theme="dark"]` on `<html>`, and the
light/dark system preference is honoured when no `data-theme` is set. All colours use
OKLCH so neutrals stay in the same hue family (~240°, ink-navy) and the orange accent
stays consistent across themes.

Accessibility: both themes pass WCAG AA for body copy, muted copy, and focus rings. Focus
is always visible (`outline: 2px solid var(--sk-accent)`). No information depends on
colour alone — Yes/No/NULL badges carry a glyph prefix.

## Extending

- **Add a new token**: declare it under `:root`, give it a dark counterpart under
  `[data-theme="dark"]` and the `prefers-color-scheme` block. Never hardcode a colour in
  a component rule — always reference `var(--sk-*)`.
- **Add a new component**: follow the `sk-<block>__<element>--<modifier>` pattern, put
  the rule in `site.css` grouped near related components, and document the class in the
  table above.
- **Add a new font**: drop the woff2 into `fonts/`, add a single `@font-face` at the top
  of `site.css`, and reference it via a new `--sk-font-*` variable. Keep `font-display:
  swap` and prefer variable woff2 when available to minimise payload.
- **Add a new icon / logo variant**: add the SVG to `img/` with a descriptive filename.
  Source logos are copied from `frontend/public/` on `main` — keep in sync if they
  evolve.

## Anti-patterns (do not reintroduce)

- Side-stripe borders thicker than 1px on callouts / cards
- Gradient text or gradient button fills
- Decorative glassmorphism on interior cards / modals — glass is reserved for
  top-level chrome (sidebar, topbar)
- Cyan-on-dark or pure black / pure white
- Monospace used as generic "tech" decoration outside of code
- Inter or JetBrains Mono
