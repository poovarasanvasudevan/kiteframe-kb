# Initial ER model

Core ownership is `organizations 1—N users|roles|knowledge_bases|audit_logs`. Users and roles form M—N through `user_roles`; roles and permissions form M—N through `role_permissions`. Knowledge bases own a recursive category tree and later own articles. Every business table repeats `organization_id` so tenant filtering is explicit and indexable.

Initial tables:

- `organizations`, `users`, `roles`, `permissions`, `role_permissions`, `user_roles`
- `refresh_tokens`, `audit_logs`
- `knowledge_bases`, `categories`

Article-phase tables:

- `articles`, `article_versions`, `tags`, `article_tags`
- `article_reviews`, `review_comments`, `attachments`
- `article_views`, `article_feedback`, `search_queries`
- `teams`, `team_users`, `knowledge_base_users`, `knowledge_base_teams`
- `notifications`

Categories use a self-referencing `parent_id`. Safe moves run in a transaction and reject self-parenting or any target discovered among the moving node's descendants.
