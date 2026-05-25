use std::sync::Arc;

use axum::{
    extract::{Path, Query},
    response::Json,
};
use clickhouse::Client;
use serde::Deserialize;

use crate::config::AppConfig;
use crate::services::tron::neo4j::{
    client::Neo4jClient, flow_graph::build_wallet_flow_graph, types::WalletFlowGraph,
};
use crate::utils::tron_address::normalize_tron_address;

#[derive(Debug, Deserialize)]
pub struct WalletGraphQuery {
    pub depth: Option<u8>,
    pub limit: Option<u64>,
}

pub async fn tron_wallet_graph(
    Path(address): Path<String>,
    Query(params): Query<WalletGraphQuery>,
) -> Result<Json<WalletFlowGraph>, String> {
    let config = AppConfig::from_env();
    let address = normalize_tron_address(&address)
        .ok_or_else(|| format!("invalid Tron wallet address: {}", address))?;

    let clickhouse = Arc::new(
        Client::default()
            .with_url(&config.clickhouse_url)
            .with_user(&config.clickhouse_user)
            .with_password(&config.clickhouse_pass)
            .with_database(&config.clickhouse_db_tron),
    );

    let neo4j = Neo4jClient::new(
        &config.neo4j_uri,
        &config.neo4j_username,
        &config.neo4j_password,
    )
    .await
    .map_err(|err| err.to_string())?;

    let graph = build_wallet_flow_graph(
        clickhouse,
        &neo4j,
        &address,
        params.depth.unwrap_or(3),
        params.limit.unwrap_or(500),
    )
    .await
    .map_err(|err| err.to_string())?;

    Ok(Json(graph))
}
