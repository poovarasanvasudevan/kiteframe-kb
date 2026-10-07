# Kiteframe interface system

## Direction

A KiteFrame operations shell: narrow dark navy icon rail, white header chrome, soft paper work surface, and compact card panels. Primary actions use navy; links and selection use blue; the brand mark uses teal green.

## Density

UI kit uses `--kf-density: 0.95` (5% tighter than prior). App shell uses `--density: 0.76` (~5% tighter than the previous 0.80 scale). Controls, type, padding, and rail width scale from these tokens.

## Units

Prefer `%` (and `px`/`rem` tokens) over `vh` / `vw`. Full-height layouts use `height: 100%` through `html → body → #root → .app-frame`.

## Typography

Use Inter for operational UI and JetBrains Mono for compact metadata. Headings stay upright and compact.

## Color

Named CSS tokens only. Navy (`--color-rail` / `--color-primary`) for shell and primary buttons. Blue (`--color-accent`) for links, focus, and active accents. Teal (`--color-brand`) for the logo mark and notification dots.

## Shape and depth

Controls use density-scaled 4–6px radii. Soft 1px borders plus a quiet card shadow separate panels from the paper background.

## Layout

Desktop: density-scaled icon rail + sticky topbar + work surface. Tablet/phone: rail becomes a labeled drawer; touch targets expand to ≥44px where required.

## Shared package

Reusable primitives live in `@kiteframe/ui` (`packages/ui`). Import `@kiteframe/ui/styles.css` before app styles.
