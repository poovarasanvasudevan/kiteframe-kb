CREATE TABLE teams (
    id BINARY(16) PRIMARY KEY,
    organization_id BINARY(16) NOT NULL,
    name VARCHAR(100) NOT NULL,
    description VARCHAR(500) NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    deleted_at TIMESTAMP(6) NULL,
    CONSTRAINT fk_teams_org FOREIGN KEY (organization_id) REFERENCES organizations(id),
    UNIQUE KEY uq_teams_org_name (organization_id, name)
);

CREATE TABLE team_users (
    team_id BINARY(16) NOT NULL,
    user_id BINARY(16) NOT NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (team_id, user_id),
    CONSTRAINT fk_team_users_team FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
    CONSTRAINT fk_team_users_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE knowledge_base_users (
    knowledge_base_id BINARY(16) NOT NULL,
    user_id BINARY(16) NOT NULL,
    can_read BOOLEAN NOT NULL DEFAULT TRUE,
    can_write BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (knowledge_base_id, user_id),
    CONSTRAINT fk_kbu_kb FOREIGN KEY (knowledge_base_id) REFERENCES knowledge_bases(id) ON DELETE CASCADE,
    CONSTRAINT fk_kbu_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE knowledge_base_teams (
    knowledge_base_id BINARY(16) NOT NULL,
    team_id BINARY(16) NOT NULL,
    can_read BOOLEAN NOT NULL DEFAULT TRUE,
    can_write BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (knowledge_base_id, team_id),
    CONSTRAINT fk_kbt_kb FOREIGN KEY (knowledge_base_id) REFERENCES knowledge_bases(id) ON DELETE CASCADE,
    CONSTRAINT fk_kbt_team FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
);

CREATE TABLE articles (
    id BINARY(16) PRIMARY KEY,
    organization_id BINARY(16) NOT NULL,
    knowledge_base_id BINARY(16) NOT NULL,
    category_id BINARY(16) NOT NULL,
    author_id BINARY(16) NOT NULL,
    current_version_id BINARY(16) NULL,
    published_version_id BINARY(16) NULL,
    title VARCHAR(240) NOT NULL,
    slug VARCHAR(260) NOT NULL,
    short_description VARCHAR(500) NULL,
    draft_content MEDIUMTEXT NOT NULL,
    status ENUM('DRAFT','IN_REVIEW','APPROVED','PUBLISHED','ARCHIVED') NOT NULL DEFAULT 'DRAFT',
    visibility ENUM('PUBLIC','PRIVATE','INTERNAL') NOT NULL DEFAULT 'INTERNAL',
    language VARCHAR(12) NOT NULL DEFAULT 'en',
    featured BOOLEAN NOT NULL DEFAULT FALSE,
    pinned BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INT NOT NULL DEFAULT 0,
    seo_title VARCHAR(255) NULL,
    seo_description VARCHAR(500) NULL,
    seo_keywords VARCHAR(500) NULL,
    canonical_url VARCHAR(2048) NULL,
    published_at TIMESTAMP(6) NULL,
    scheduled_publish_at TIMESTAMP(6) NULL,
    last_reviewed_at TIMESTAMP(6) NULL,
    next_review_at TIMESTAMP(6) NULL,
    lock_version BIGINT UNSIGNED NOT NULL DEFAULT 0,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    deleted_at TIMESTAMP(6) NULL,
    CONSTRAINT fk_articles_org FOREIGN KEY (organization_id) REFERENCES organizations(id),
    CONSTRAINT fk_articles_kb FOREIGN KEY (knowledge_base_id) REFERENCES knowledge_bases(id),
    CONSTRAINT fk_articles_category FOREIGN KEY (category_id) REFERENCES categories(id),
    CONSTRAINT fk_articles_author FOREIGN KEY (author_id) REFERENCES users(id),
    UNIQUE KEY uq_articles_kb_slug_language (knowledge_base_id, slug, language),
    KEY idx_articles_org_status_updated (organization_id, status, updated_at),
    KEY idx_articles_kb_category_status (knowledge_base_id, category_id, status),
    KEY idx_articles_schedule (status, scheduled_publish_at),
    FULLTEXT KEY ft_articles_search (title, short_description, draft_content)
);

CREATE TABLE article_versions (
    id BINARY(16) PRIMARY KEY,
    organization_id BINARY(16) NOT NULL,
    article_id BINARY(16) NOT NULL,
    version_number INT UNSIGNED NOT NULL,
    title VARCHAR(240) NOT NULL,
    content MEDIUMTEXT NOT NULL,
    metadata JSON NOT NULL,
    author_id BINARY(16) NOT NULL,
    change_summary VARCHAR(500) NULL,
    is_published BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_versions_org FOREIGN KEY (organization_id) REFERENCES organizations(id),
    CONSTRAINT fk_versions_article FOREIGN KEY (article_id) REFERENCES articles(id),
    CONSTRAINT fk_versions_author FOREIGN KEY (author_id) REFERENCES users(id),
    UNIQUE KEY uq_versions_article_number (article_id, version_number),
    KEY idx_versions_article_created (article_id, created_at)
);

ALTER TABLE articles
    ADD CONSTRAINT fk_articles_current_version FOREIGN KEY (current_version_id) REFERENCES article_versions(id),
    ADD CONSTRAINT fk_articles_published_version FOREIGN KEY (published_version_id) REFERENCES article_versions(id);

CREATE TABLE tags (
    id BINARY(16) PRIMARY KEY,
    organization_id BINARY(16) NOT NULL,
    name VARCHAR(80) NOT NULL,
    slug VARCHAR(100) NOT NULL,
    CONSTRAINT fk_tags_org FOREIGN KEY (organization_id) REFERENCES organizations(id),
    UNIQUE KEY uq_tags_org_slug (organization_id, slug)
);

CREATE TABLE article_tags (
    article_id BINARY(16) NOT NULL,
    tag_id BINARY(16) NOT NULL,
    PRIMARY KEY (article_id, tag_id),
    CONSTRAINT fk_article_tags_article FOREIGN KEY (article_id) REFERENCES articles(id) ON DELETE CASCADE,
    CONSTRAINT fk_article_tags_tag FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

CREATE TABLE article_reviews (
    id BINARY(16) PRIMARY KEY,
    organization_id BINARY(16) NOT NULL,
    article_id BINARY(16) NOT NULL,
    version_id BINARY(16) NOT NULL,
    requested_by BINARY(16) NOT NULL,
    reviewer_id BINARY(16) NULL,
    status ENUM('PENDING','APPROVED','CHANGES_REQUESTED','REJECTED') NOT NULL DEFAULT 'PENDING',
    decision_comment VARCHAR(2000) NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    decided_at TIMESTAMP(6) NULL,
    CONSTRAINT fk_reviews_org FOREIGN KEY (organization_id) REFERENCES organizations(id),
    CONSTRAINT fk_reviews_article FOREIGN KEY (article_id) REFERENCES articles(id),
    CONSTRAINT fk_reviews_version FOREIGN KEY (version_id) REFERENCES article_versions(id),
    CONSTRAINT fk_reviews_requester FOREIGN KEY (requested_by) REFERENCES users(id),
    CONSTRAINT fk_reviews_reviewer FOREIGN KEY (reviewer_id) REFERENCES users(id),
    KEY idx_reviews_org_status_created (organization_id, status, created_at)
);

CREATE TABLE review_comments (
    id BINARY(16) PRIMARY KEY,
    review_id BINARY(16) NOT NULL,
    user_id BINARY(16) NOT NULL,
    body VARCHAR(4000) NOT NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_review_comments_review FOREIGN KEY (review_id) REFERENCES article_reviews(id) ON DELETE CASCADE,
    CONSTRAINT fk_review_comments_user FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE attachments (
    id BINARY(16) PRIMARY KEY,
    organization_id BINARY(16) NOT NULL,
    article_id BINARY(16) NOT NULL,
    uploaded_by BINARY(16) NOT NULL,
    filename VARCHAR(255) NOT NULL,
    mime_type VARCHAR(120) NOT NULL,
    size_bytes BIGINT UNSIGNED NOT NULL,
    storage_key VARCHAR(1024) NOT NULL,
    public_url VARCHAR(2048) NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_attachments_org FOREIGN KEY (organization_id) REFERENCES organizations(id),
    CONSTRAINT fk_attachments_article FOREIGN KEY (article_id) REFERENCES articles(id),
    CONSTRAINT fk_attachments_uploader FOREIGN KEY (uploaded_by) REFERENCES users(id),
    KEY idx_attachments_article (article_id)
);

CREATE TABLE article_views (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    organization_id BINARY(16) NOT NULL,
    article_id BINARY(16) NOT NULL,
    visitor_hash BINARY(32) NULL,
    viewed_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_views_org FOREIGN KEY (organization_id) REFERENCES organizations(id),
    CONSTRAINT fk_views_article FOREIGN KEY (article_id) REFERENCES articles(id),
    KEY idx_views_org_date (organization_id, viewed_at),
    KEY idx_views_article_date (article_id, viewed_at)
);

CREATE TABLE article_feedback (
    id BINARY(16) PRIMARY KEY,
    organization_id BINARY(16) NOT NULL,
    article_id BINARY(16) NOT NULL,
    helpful BOOLEAN NOT NULL,
    comment VARCHAR(2000) NULL,
    visitor_hash BINARY(32) NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_feedback_org FOREIGN KEY (organization_id) REFERENCES organizations(id),
    CONSTRAINT fk_feedback_article FOREIGN KEY (article_id) REFERENCES articles(id),
    KEY idx_feedback_org_date (organization_id, created_at),
    KEY idx_feedback_article_helpful (article_id, helpful)
);

CREATE TABLE search_queries (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    organization_id BINARY(16) NOT NULL,
    knowledge_base_id BINARY(16) NULL,
    user_id BINARY(16) NULL,
    query VARCHAR(500) NOT NULL,
    results_count INT UNSIGNED NOT NULL,
    selected_article_id BINARY(16) NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_search_org FOREIGN KEY (organization_id) REFERENCES organizations(id),
    CONSTRAINT fk_search_kb FOREIGN KEY (knowledge_base_id) REFERENCES knowledge_bases(id),
    CONSTRAINT fk_search_user FOREIGN KEY (user_id) REFERENCES users(id),
    CONSTRAINT fk_search_selected FOREIGN KEY (selected_article_id) REFERENCES articles(id),
    KEY idx_search_org_date (organization_id, created_at),
    KEY idx_search_failed (organization_id, results_count, created_at)
);

CREATE TABLE notifications (
    id BINARY(16) PRIMARY KEY,
    organization_id BINARY(16) NOT NULL,
    user_id BINARY(16) NOT NULL,
    event_type VARCHAR(80) NOT NULL,
    title VARCHAR(255) NOT NULL,
    body VARCHAR(1000) NULL,
    resource_type VARCHAR(80) NULL,
    resource_id BINARY(16) NULL,
    read_at TIMESTAMP(6) NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_notifications_org FOREIGN KEY (organization_id) REFERENCES organizations(id),
    CONSTRAINT fk_notifications_user FOREIGN KEY (user_id) REFERENCES users(id),
    KEY idx_notifications_user_read_created (user_id, read_at, created_at)
);
