use std::env;

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub database_max_connections: u32,
    pub jwt_secret: String,
    pub jwt_access_expiration_seconds: i64,
    pub jwt_refresh_expiration_seconds: i64,
    pub port: u16,
    pub cors_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let jwt_secret = env::var("JWT_SECRET").context("JWT_SECRET is required")?;
        anyhow::ensure!(
            jwt_secret.len() >= 32,
            "JWT_SECRET must be at least 32 characters"
        );
        Ok(Self {
            database_url: env::var("DATABASE_URL").context("DATABASE_URL is required")?,
            database_max_connections: parse("DATABASE_MAX_CONNECTIONS", 10)?,
            jwt_secret,
            jwt_access_expiration_seconds: parse("JWT_ACCESS_EXPIRATION_SECONDS", 900)?,
            jwt_refresh_expiration_seconds: parse("JWT_REFRESH_EXPIRATION_SECONDS", 2_592_000)?,
            port: parse("PORT", 8080)?,
            cors_origins: env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:5173".into())
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
        })
    }
}

fn parse<T>(name: &str, default: T) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    env::var(name)
        .map(|value| value.parse().context(format!("invalid {name}")))
        .unwrap_or(Ok(default))
}
