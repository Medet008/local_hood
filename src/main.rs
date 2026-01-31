use axum::{
    http::{header, Method},
    middleware as axum_middleware,
    routing::get,
    Json, Router,
};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use localhood_backend::{
    api,
    config::Config,
    middleware::{auth_middleware, AppState},
    ApiDoc,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Загружаем .env файл
    dotenvy::dotenv().ok();

    // Инициализируем логирование
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "localhood_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Загружаем конфигурацию
    let config = Config::from_env().expect("Failed to load configuration");

    tracing::info!("Starting LocalHood Backend...");
    tracing::info!("Connecting to database...");

    // Подключаемся к базе данных
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    tracing::info!("Connected to database successfully");

    // Запускаем миграции
    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    tracing::info!("Migrations completed");

    // Создаём состояние приложения
    let state = AppState {
        pool: pool.clone(),
        config: config.clone(),
    };

    // Настраиваем CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT]);

    // Создаём роутер
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api/v1", api::routes())
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    // Запускаем сервер
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn root() -> Json<serde_json::Value> {
    Json(json!({
        "name": "LocalHood API",
        "version": "1.0.0",
        "description": "Backend API for LocalHood - residential complex management platform"
    }))
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
