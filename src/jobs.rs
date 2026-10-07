use std::time::Duration;

use sqlx::MySqlPool;
use tracing::{error, info};
use uuid::Uuid;

pub fn spawn(pool: MySqlPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(error) = publish_scheduled(&pool).await {
                error!(?error, "scheduled publishing job failed");
            }
        }
    });
}

async fn publish_scheduled(pool: &MySqlPool) -> anyhow::Result<()> {
    let candidates = sqlx::query_as::<_, (Uuid, Uuid, Uuid)>("SELECT id, organization_id, current_version_id FROM articles WHERE status = 'APPROVED' AND scheduled_publish_at IS NOT NULL AND scheduled_publish_at <= CURRENT_TIMESTAMP(6) ORDER BY scheduled_publish_at LIMIT 50")
        .fetch_all(pool).await?;
    for (article_id, organization_id, version_id) in candidates {
        let mut tx = pool.begin().await?;
        let status = sqlx::query_scalar::<_, String>(
            "SELECT status FROM articles WHERE id = ? AND organization_id = ? FOR UPDATE",
        )
        .bind(article_id)
        .bind(organization_id)
        .fetch_one(&mut *tx)
        .await?;
        if status != "APPROVED" {
            tx.rollback().await?;
            continue;
        }
        sqlx::query(
            "UPDATE article_versions SET is_published = TRUE WHERE id = ? AND article_id = ?",
        )
        .bind(version_id)
        .bind(article_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE articles SET status = 'PUBLISHED', published_version_id = ?, published_at = CURRENT_TIMESTAMP(6), scheduled_publish_at = NULL WHERE id = ? AND status = 'APPROVED'")
            .bind(version_id).bind(article_id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO audit_logs (id, organization_id, action, resource_type, resource_id) VALUES (?, ?, 'ARTICLE_PUBLISHED', 'article', ?)")
            .bind(Uuid::new_v4()).bind(organization_id).bind(article_id).execute(&mut *tx).await?;
        tx.commit().await?;
        info!(%article_id, "scheduled article published");
    }
    Ok(())
}
