# Kiteframe KBMS

Kiteframe is a tenant-safe knowledge operations platform built with Rust/Axum, MySQL, Bun, and React.

## Run locally

1. Copy `.env.example` to `.env` and replace `JWT_SECRET`.
2. Start MySQL: `docker compose up -d mysql`.
3. Seed development data: `cargo run --bin seed`.
4. Start the API: `cargo run --bin kb-management`.
5. In another terminal, run `cd frontend && bun install && bun run dev`.

Swagger UI is available at [http://localhost:8080/docs](http://localhost:8080/docs). The web application is at [http://localhost:5173](http://localhost:5173).

Core product routes include `/dashboard`, `/knowledge`, `/articles`, `/reviews`, `/analytics`, and `/people`. The seeded public help center is `/help/product-help`.

The API runs migrations automatically on startup. Development seed credentials come from `SEED_ADMIN_EMAIL` and `SEED_PASSWORD`; change both in `.env`. The seed also creates editor, reviewer, and viewer accounts, teams/roles, nested content structure, and 20 published example articles. Never use seed credentials in production.

## Verification

```sh
cargo fmt --check
cargo test
cd frontend && bun run build
docker compose config
```

## Implemented workflows

- Tenant-scoped knowledge bases and recursive categories
- Article authoring with TipTap and optimistic autosave
- Immutable versions, review decisions, publishing, scheduling, archiving, and restoration
- Permission-safe public knowledge base, search, article feedback, and view tracking
- Analytics, users, teams, audit history, notifications, and JSON import/export

## UI kit (`@kiteframe/ui`)

Reusable React components live in [`packages/ui`](packages/ui) and are consumed by the web app via the Bun workspace.

```tsx
import '@kiteframe/ui/styles.css'
import { Button, CommandK, DataTable, Sidebar, useCommandKShortcut } from '@kiteframe/ui'
```

Includes button, text field, select, search box, avatar, icon, typography, chip, card, tabs, accordion, popover, dialog, table, data table, sidebar, and Ctrl/⌘K command palette. See [`packages/ui/README.md`](packages/ui/README.md) for reuse in other projects.

See `ARCHITECTURE.md` for boundaries and delivery phases, `docs/ER_MODEL.md` for the schema map, and `PRODUCT.md` / `DESIGN.md` for durable product and interface decisions.
