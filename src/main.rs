mod analytics;
mod app;
mod articles;
mod auth;
mod categories;
mod config;
mod error;
mod jobs;
mod knowledge_bases;
mod management;
mod public_kb;
mod services;
mod shared;

use std::net::SocketAddr;

use config::Config;
use sqlx::mysql::MySqlPoolOptions;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    let config = Config::from_env()?;
    let pool = MySqlPoolOptions::new()
        .max_connections(config.database_max_connections)
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!().run(&pool).await?;
    jobs::spawn(pool.clone());

    let address = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(address).await?;
    info!(%address, "knowledge base API listening");
    axum::serve(listener, app::router(app::AppState::new(pool, config))).await?;
    Ok(())
}
