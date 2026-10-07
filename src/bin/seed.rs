use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use sqlx::mysql::MySqlPoolOptions;
use uuid::Uuid;

const PERMISSIONS: &[&str] = &[
    "kb.read",
    "kb.create",
    "kb.update",
    "kb.delete",
    "article.read",
    "article.create",
    "article.update",
    "article.delete",
    "article.publish",
    "article.review",
    "article.archive",
    "article.restore",
    "category.manage",
    "users.manage",
    "roles.manage",
    "analytics.read",
    "settings.manage",
    "audit.read",
];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")?;
    let password = std::env::var("SEED_PASSWORD").unwrap_or_else(|_| "ChangeMe123!".into());
    let pool = MySqlPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await?;
    sqlx::migrate!().run(&pool).await?;
    let mut tx = pool.begin().await?;
    let organization_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name, slug) VALUES (?, 'Northwind Documentation', 'northwind') ON DUPLICATE KEY UPDATE name = VALUES(name)")
        .bind(organization_id).execute(&mut *tx).await?;
    let organization_id =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM organizations WHERE slug = 'northwind'")
            .fetch_one(&mut *tx)
            .await?;

    for permission in PERMISSIONS {
        sqlx::query("INSERT INTO permissions (id, name) VALUES (?, ?) ON DUPLICATE KEY UPDATE name = VALUES(name)")
            .bind(Uuid::new_v4()).bind(permission).execute(&mut *tx).await?;
    }
    let roles = ["Organization Admin", "Editor", "Reviewer", "Viewer"];
    for role in roles {
        sqlx::query("INSERT INTO roles (id, organization_id, name) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE name = VALUES(name)")
            .bind(Uuid::new_v4()).bind(organization_id).bind(role).execute(&mut *tx).await?;
    }
    let admin_role = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM roles WHERE organization_id = ? AND name = 'Organization Admin'",
    )
    .bind(organization_id)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("INSERT IGNORE INTO role_permissions (role_id, permission_id) SELECT ?, id FROM permissions").bind(admin_role).execute(&mut *tx).await?;
    grant_role(
        &mut tx,
        organization_id,
        "Editor",
        &[
            "kb.read",
            "article.read",
            "article.create",
            "article.update",
            "category.manage",
        ],
    )
    .await?;
    grant_role(
        &mut tx,
        organization_id,
        "Reviewer",
        &[
            "kb.read",
            "article.read",
            "article.review",
            "article.publish",
        ],
    )
    .await?;
    grant_role(
        &mut tx,
        organization_id,
        "Viewer",
        &["kb.read", "article.read"],
    )
    .await?;

    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
        .to_string();
    let people = [
        (
            "Admin User",
            std::env::var("SEED_ADMIN_EMAIL").unwrap_or_else(|_| "admin@example.com".into()),
            "Organization Admin",
        ),
        ("Editorial User", "editor@example.com".into(), "Editor"),
        ("Review User", "reviewer@example.com".into(), "Reviewer"),
        ("Read Only User", "viewer@example.com".into(), "Viewer"),
    ];
    let mut admin_id = Uuid::nil();
    for (name, email, role) in people {
        sqlx::query("INSERT INTO users (id, organization_id, name, email, password_hash, status, email_verified_at) VALUES (?, ?, ?, ?, ?, 'ACTIVE', CURRENT_TIMESTAMP(6)) ON DUPLICATE KEY UPDATE name = VALUES(name), password_hash = VALUES(password_hash), status = 'ACTIVE'")
            .bind(Uuid::new_v4()).bind(organization_id).bind(name).bind(&email).bind(&password_hash).execute(&mut *tx).await?;
        let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = ?")
            .bind(&email)
            .fetch_one(&mut *tx)
            .await?;
        let role_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM roles WHERE organization_id = ? AND name = ?",
        )
        .bind(organization_id)
        .bind(role)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("INSERT IGNORE INTO user_roles (user_id, role_id) VALUES (?, ?)")
            .bind(user_id)
            .bind(role_id)
            .execute(&mut *tx)
            .await?;
        if role == "Organization Admin" {
            admin_id = user_id;
        }
    }

    let kb_id = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_bases (id, organization_id, name, slug, description, visibility, status, language, created_by) VALUES (?, ?, 'Product Help', 'product-help', 'Answers for setup, accounts, billing, and troubleshooting.', 'PUBLIC', 'ACTIVE', 'en', ?) ON DUPLICATE KEY UPDATE description = VALUES(description)")
        .bind(kb_id).bind(organization_id).bind(admin_id).execute(&mut *tx).await?;
    let kb_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM knowledge_bases WHERE organization_id = ? AND slug = 'product-help'",
    )
    .bind(organization_id)
    .fetch_one(&mut *tx)
    .await?;
    let categories = ["Getting Started", "Account", "Billing", "Troubleshooting"];
    for (index, name) in categories.iter().enumerate() {
        let slug = name.to_lowercase().replace(' ', "-");
        sqlx::query("INSERT INTO categories (id, organization_id, knowledge_base_id, name, slug, description, visibility, sort_order, created_by) VALUES (?, ?, ?, ?, ?, ?, 'PUBLIC', ?, ?) ON DUPLICATE KEY UPDATE name = VALUES(name)")
            .bind(Uuid::new_v4()).bind(organization_id).bind(kb_id).bind(name).bind(&slug).bind(format!("Guides for {name}." )).bind(index as u32).bind(admin_id).execute(&mut *tx).await?;
    }
    let category_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM categories WHERE knowledge_base_id = ? ORDER BY sort_order",
    )
    .bind(kb_id)
    .fetch_all(&mut *tx)
    .await?;
    for index in 1..=20 {
        let slug = format!("example-guide-{index}");
        let article_id = Uuid::new_v4();
        let version_id = Uuid::new_v4();
        let title = format!("Example guide {index}");
        let content = format!(
            "<h2>Guide {index}</h2><p>This seeded article demonstrates a published knowledge-base answer.</p>"
        );
        sqlx::query("INSERT INTO articles (id, organization_id, knowledge_base_id, category_id, author_id, current_version_id, published_version_id, title, slug, short_description, draft_content, status, visibility, language, published_at) VALUES (?, ?, ?, ?, ?, NULL, NULL, ?, ?, ?, ?, 'PUBLISHED', 'PUBLIC', 'en', CURRENT_TIMESTAMP(6)) ON DUPLICATE KEY UPDATE title = VALUES(title)")
            .bind(article_id).bind(organization_id).bind(kb_id).bind(category_ids[(index - 1) % category_ids.len()]).bind(admin_id).bind(&title).bind(&slug).bind(format!("A concise example answer for guide {index}." )).bind(&content).execute(&mut *tx).await?;
        let stored_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM articles WHERE knowledge_base_id = ? AND slug = ? AND language = 'en'",
        )
        .bind(kb_id)
        .bind(&slug)
        .fetch_one(&mut *tx)
        .await?;
        let existing_version = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM article_versions WHERE article_id = ? AND version_number = 1",
        )
        .bind(stored_id)
        .fetch_optional(&mut *tx)
        .await?;
        let stored_version = if let Some(id) = existing_version {
            id
        } else {
            sqlx::query("INSERT INTO article_versions (id, organization_id, article_id, version_number, title, content, metadata, author_id, change_summary, is_published) VALUES (?, ?, ?, 1, ?, ?, JSON_OBJECT('visibility','PUBLIC','short_description',?), ?, 'Initial seed', TRUE)")
                .bind(version_id).bind(organization_id).bind(stored_id).bind(&title).bind(&content).bind(format!("A concise example answer for guide {index}." )).bind(admin_id).execute(&mut *tx).await?;
            version_id
        };
        sqlx::query(
            "UPDATE articles SET current_version_id = ?, published_version_id = ? WHERE id = ?",
        )
        .bind(stored_version)
        .bind(stored_version)
        .bind(stored_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    println!(
        "Seed complete. Admin: {}",
        std::env::var("SEED_ADMIN_EMAIL").unwrap_or_else(|_| "admin@example.com".into())
    );
    Ok(())
}

async fn grant_role(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    organization_id: Uuid,
    role: &str,
    permissions: &[&str],
) -> anyhow::Result<()> {
    let role_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM roles WHERE organization_id = ? AND name = ?",
    )
    .bind(organization_id)
    .bind(role)
    .fetch_one(&mut **tx)
    .await?;
    for permission in permissions {
        sqlx::query("INSERT IGNORE INTO role_permissions (role_id, permission_id) SELECT ?, id FROM permissions WHERE name = ?").bind(role_id).bind(permission).execute(&mut **tx).await?;
    }
    Ok(())
}
