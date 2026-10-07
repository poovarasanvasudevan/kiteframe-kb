# Architecture

## Repository

```text
.
├── src/                     Rust API, domain-oriented modules
├── migrations/              ordered MySQL migrations
├── frontend/                Bun + React application
├── docs/                    ER model and API conventions
├── Dockerfile.backend
├── docker-compose.yml
├── PRODUCT.md               durable product truth
└── DESIGN.md                durable interface rules
```

Backend dependency direction is route → handler → service → repository → MySQL. The foundation keeps small domains together until their business logic requires splitting; transport code must not own state-transition rules.

## Security boundary

Access JWTs carry the authenticated user, organization, and effective permissions. Extractors reject invalid tokens before handlers run. Every repository query includes the identity-derived `organization_id`; tenant IDs are not accepted from request bodies. Passwords use Argon2id. Refresh tokens are stored only as hashes, rotated as a family, and revocable.

## Article lifecycle

```text
DRAFT ──submit──> IN_REVIEW ──approve──> APPROVED ──publish──> PUBLISHED
  ^                    │ request changes / reject                    │
  │                    └─────────────────────────────────────────────┘
  └──────────── restore draft <──────── ARCHIVED <──── archive ─────┘
```

Publishing happens in one transaction: lock article, validate permission and transition, snapshot an immutable version, point the article at it, set publication time, and append an audit event. Published snapshots are never updated.

## API conventions

- Prefix: `/api/v1`.
- JSON envelopes use `{ data, meta, error }`.
- Validation failures are 422, authentication 401, authorization 403, absent tenant-scoped resources 404, and uniqueness/state conflicts 409.
- List endpoints use `page`/`page_size` until a measured high-volume feed warrants cursors.
- OpenAPI JSON is served at `/api-docs/openapi.json`; Swagger UI is `/docs`.

## Delivery phases

1. Foundation: runtime, migrations, authentication/RBAC, organizations/users, app shell.
2. Knowledge structure: knowledge bases, category tree, safe move/reorder, settings.
3. Authoring: article CRUD, editor, autosave, tags, uploads, preview.
4. Governance: versions, reviews, publication, scheduling, archiving.
5. Public delivery: help center, permission-safe search, feedback, SEO.
6. Insight: views, search and feedback analytics, dashboards.
7. Enterprise: teams, granular resource grants, notifications, audit UI, import/export, localization.

### Phase 2 implementation note

Category reads and writes are scoped by both authenticated `organization_id` and `knowledge_base_id`. Moves lock the affected records, reject cross-knowledge-base parents, use a recursive CTE to prevent cycles, open a sibling position, and commit the new parent/order atomically. Reorder accepts siblings only and retains keyboard-accessible up/down actions alongside drag interaction.

## Implemented phase coverage

- **Foundation:** JWT access tokens, rotating hashed refresh tokens, Argon2id verification, RBAC, tenant extractors, request tracing, Docker configuration, OpenAPI shell, responsive app shell, and reproducible seed data.
- **Knowledge structure:** knowledge bases, settings API, recursive categories, protected deletion, move/reorder transactions, and category workspace.
- **Authoring:** article CRUD, TipTap editor, optimistic concurrency, debounced autosave, immutable version snapshots, metadata/language fields, and storage/search provider seams.
- **Governance:** submit, approve, request-changes, publish, schedule worker, archive and restore transitions; review and audit records; in-app notification records.
- **Public delivery:** public-only knowledge-base and article queries, immutable published content, full-text/prefix search, search telemetry, feedback throttling, and responsive reader/search surfaces.
- **Insight:** organization-scoped article, view, search, failed-search, review, and feedback metrics with a connected dashboard.
- **Enterprise:** users, roles, permissions, teams, resource-grant schema, audit viewer, notification feed API, JSON import/export, and language-aware content architecture.

## Replaceable service boundaries

`StorageProvider` defines upload/delete/URL behavior with a traversal-safe local implementation and an S3-compatible future seam. `SearchProvider` defines the search boundary; current public search uses permission-safe MySQL full-text queries. Background publishing runs as an idempotent Tokio worker and can move behind a dedicated queue without changing article transitions.
