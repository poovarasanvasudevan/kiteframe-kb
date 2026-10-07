# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Rust/Axum/Tokio/SQLx with MySQL 8; Bun, React, TypeScript, Vite, Tailwind, TanStack Query, React Hook Form, and Zod.

## Users

Organization administrators, knowledge-base administrators, editors, authors, reviewers, and viewers managing public, private, and internal documentation.

## Product Purpose

Give organizations one tenant-safe workspace to structure, review, publish, find, and measure knowledge. Success means content teams can move an article from draft to a trustworthy published version without losing ownership, history, or access control.

## Positioning

Editorial governance and public help-center delivery share the same content model, permission boundary, and audit trail.

## Operating Context

Frequent desktop work includes triaging drafts, navigating nested categories, reviewing changes, scheduling publication, and interpreting search and feedback signals. Public readers use responsive help centers and search.

## Capabilities and Constraints

- Organization-aware multi-tenancy; organization context comes from authentication, never client-supplied IDs.
- Backend-enforced RBAC with granular permissions.
- Immutable published article versions and explicit lifecycle transitions.
- English first, with language-aware schemas and APIs.
- Local file storage and MySQL full-text search first, behind replaceable boundaries.
- WCAG-minded, keyboard-operable responsive interfaces.

## Brand Commitments

Original product identity. The working UI uses “KiteFrame” as the product name.

## Evidence on Hand

The master requirements are the only source material. No customer metrics, testimonials, logos, or production content exist and must not be fabricated.

## Product Principles

- Tenant isolation is part of every query, not a controller afterthought.
- Published knowledge is deliberate and immutable.
- Dense workflows remain calm, legible, and keyboard accessible.
- Operational signals should lead to an obvious next action.
- Extensibility follows demonstrated seams rather than speculative machinery.

## Accessibility & Inclusion

WCAG principles apply: semantic HTML, visible focus, keyboard navigation, sufficient contrast, accessible dialogs, and non-color-only state communication.
