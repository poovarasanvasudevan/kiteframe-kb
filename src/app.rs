use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    http::{HeaderValue, Method},
    routing::{get, post},
};
use sqlx::MySqlPool;
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    analytics, articles, auth, categories, config::Config, knowledge_bases, management, public_kb,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: MySqlPool,
    pub config: Arc<Config>,
    pub jwt: auth::JwtService,
}

impl AppState {
    pub fn new(pool: MySqlPool, config: Config) -> Self {
        let jwt = auth::JwtService::new(
            config.jwt_secret.as_bytes(),
            config.jwt_access_expiration_seconds,
        );
        Self {
            pool,
            config: Arc::new(config),
            jwt,
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(auth::login, knowledge_bases::list, knowledge_bases::create, knowledge_bases::get,
        categories::list, categories::create, categories::move_category, categories::reorder),
    components(schemas(auth::LoginRequest, auth::LoginResponse, auth::AuthenticatedUser,
        knowledge_bases::KnowledgeBase, knowledge_bases::CreateKnowledgeBase, knowledge_bases::Visibility,
        categories::Category, categories::CreateCategory, categories::MoveCategory, categories::ReorderCategories,
        articles::Article, articles::ArticleVersion, articles::CreateArticle, articles::UpdateArticle,
        articles::ArticleVisibility, articles::DecisionInput, analytics::AnalyticsSummary,
        public_kb::PublicKnowledgeBase, public_kb::PublicArticle, public_kb::FeedbackInput)),
    modifiers(&SecurityAddon),
    tags((name = "KBMS", description = "Tenant-safe knowledge management API"))
)]
struct ApiDoc;

struct SecurityAddon;
impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

pub fn router(state: AppState) -> Router {
    let cors = state
        .config
        .cors_origins
        .iter()
        .filter_map(|origin| origin.parse::<HeaderValue>().ok())
        .fold(CorsLayer::new(), |layer, origin| layer.allow_origin(origin))
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ])
        .max_age(Duration::from_secs(3600));

    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/refresh", post(auth::refresh))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route(
            "/api/v1/knowledge-bases",
            get(knowledge_bases::list).post(knowledge_bases::create),
        )
        .route(
            "/api/v1/knowledge-bases/{id}",
            get(knowledge_bases::get).patch(knowledge_bases::update),
        )
        .route(
            "/api/v1/categories/by-knowledge-base/{knowledge_base_id}",
            get(categories::list),
        )
        .route("/api/v1/categories", post(categories::create))
        .route(
            "/api/v1/categories/{id}",
            axum::routing::patch(categories::update).delete(categories::delete),
        )
        .route(
            "/api/v1/categories/{id}/move",
            post(categories::move_category),
        )
        .route("/api/v1/categories/reorder", post(categories::reorder))
        .route(
            "/api/v1/articles",
            get(articles::list).post(articles::create),
        )
        .route(
            "/api/v1/articles/{id}",
            get(articles::get).patch(articles::update),
        )
        .route("/api/v1/articles/{id}/versions", get(articles::versions))
        .route(
            "/api/v1/articles/{id}/submit-review",
            post(articles::submit_review),
        )
        .route("/api/v1/articles/{id}/approve", post(articles::approve))
        .route("/api/v1/articles/{id}/reject", post(articles::reject))
        .route("/api/v1/articles/{id}/publish", post(articles::publish))
        .route("/api/v1/articles/{id}/archive", post(articles::archive))
        .route("/api/v1/articles/{id}/restore", post(articles::restore))
        .route("/api/v1/analytics", get(analytics::summary))
        .route("/api/v1/users", get(management::users))
        .route(
            "/api/v1/teams",
            get(management::teams).post(management::create_team),
        )
        .route("/api/v1/audit-logs", get(management::audit_logs))
        .route("/api/v1/notifications", get(management::notifications))
        .route("/api/v1/import/articles", post(management::import_articles))
        .route("/api/v1/export/articles", get(management::export_articles))
        .route("/api/v1/public/{kb_slug}", get(public_kb::knowledge_base))
        .route(
            "/api/v1/public/{kb_slug}/articles/{article_slug}",
            get(public_kb::article),
        )
        .route("/api/v1/public/{kb_slug}/search", get(public_kb::search))
        .route(
            "/api/v1/public/articles/{id}/feedback",
            post(public_kb::feedback),
        )
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
