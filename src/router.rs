use crate::handlers::{dashboard, health, status, tron_graph};
use axum::{
    Router,
    routing::{get, post},
};

pub fn build_router() -> Router {
    Router::new()
        .route("/", get(dashboard::dashboard))
        .route("/health", get(health::health_check))
        .route("/status", get(status::status))
        .route(
            "/tron/wallet/{address}/graph",
            get(tron_graph::tron_wallet_graph),
        )
        .route(
            "/tron/wallet/{address}/neo4j/import",
            post(tron_graph::tron_wallet_graph),
        )
        .route(
            "/api/tron/wallet/{address}/graph",
            get(tron_graph::tron_wallet_graph),
        )
        .route(
            "/api/tron/wallet/{address}/neo4j/import",
            post(tron_graph::tron_wallet_graph),
        )
}
