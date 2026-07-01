use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use clickhouse::Client;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::services::tron::neo4j::{
    client::Neo4jClient, flow_graph::build_wallet_flow_graph, types::WalletFlowGraph,
};
use crate::services::tron::wallet_aml_risk::{WalletAmlRiskAssessment, assess_wallet_fingerprint};
use crate::services::tron::wallet_fingerprint::{WalletFingerprint, build_wallet_fingerprint};
use crate::utils::tron_address::normalize_tron_address;

#[derive(Debug, Deserialize)]
pub struct WalletInvestigationQuery {
    pub depth: Option<u8>,
    pub limit: Option<u64>,
    pub window_days: Option<u16>,
    pub top_counterparties: Option<usize>,
    pub max_events: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct WalletInvestigation {
    pub address: String,
    pub graph: WalletFlowGraph,
    pub fingerprint: WalletFingerprint,
    pub aml_risk: WalletAmlRiskAssessment,
    pub data_quality: InvestigationDataQuality,
}

#[derive(Debug, Serialize)]
pub struct InvestigationDataQuality {
    pub graph_depth: u8,
    pub graph_edge_limit: u64,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub fingerprint_event_limit: u64,
    pub fingerprint_is_truncated: bool,
    pub observed_transfers: u64,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct WalletInvestigationError {
    status: StatusCode,
    message: String,
}

impl WalletInvestigationError {
    fn bad_request(message: String) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message,
        }
    }

    fn internal(err: anyhow::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("{err:#}"),
        }
    }
}

impl IntoResponse for WalletInvestigationError {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}

pub async fn tron_wallet_investigation(
    Path(address): Path<String>,
    Query(params): Query<WalletInvestigationQuery>,
) -> Result<Json<WalletInvestigation>, WalletInvestigationError> {
    let config = AppConfig::from_env();
    let address = normalize_tron_address(&address).ok_or_else(|| {
        WalletInvestigationError::bad_request(format!("invalid Tron wallet address: {address}"))
    })?;

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
    .map_err(WalletInvestigationError::internal)?;

    let graph_depth = params.depth.unwrap_or(3).clamp(1, 6);
    let graph_edge_limit = params.limit.unwrap_or(500).clamp(1, 2_000);

    let graph = build_wallet_flow_graph(
        clickhouse.clone(),
        &neo4j,
        &address,
        graph_depth,
        graph_edge_limit,
    )
    .await
    .map_err(WalletInvestigationError::internal)?;

    let fingerprint = build_wallet_fingerprint(
        clickhouse,
        &address,
        params.window_days,
        params.top_counterparties,
        params.max_events,
    )
    .await
    .map_err(WalletInvestigationError::internal)?;

    let aml_risk = assess_wallet_fingerprint(&fingerprint);
    let data_quality = build_data_quality(&graph, &fingerprint, graph_depth, graph_edge_limit);

    Ok(Json(WalletInvestigation {
        address,
        graph,
        fingerprint,
        aml_risk,
        data_quality,
    }))
}

fn build_data_quality(
    graph: &WalletFlowGraph,
    fingerprint: &WalletFingerprint,
    graph_depth: u8,
    graph_edge_limit: u64,
) -> InvestigationDataQuality {
    let mut warnings = Vec::new();

    if fingerprint.is_truncated {
        warnings.push("fingerprint_event_sample_truncated".to_string());
    }

    if fingerprint.flows.total_transfers == 0 {
        warnings.push("no_observed_flow_history".to_string());
    } else if fingerprint.flows.total_transfers < 3 {
        warnings.push("low_observed_flow_volume".to_string());
    }

    if graph.edges.len() as u64 >= graph_edge_limit {
        warnings.push("graph_edge_limit_reached".to_string());
    }

    if graph_depth >= 6 {
        warnings.push("graph_depth_capped".to_string());
    }

    InvestigationDataQuality {
        graph_depth,
        graph_edge_limit,
        graph_nodes: graph.nodes.len(),
        graph_edges: graph.edges.len(),
        fingerprint_event_limit: fingerprint.sampled_event_limit,
        fingerprint_is_truncated: fingerprint.is_truncated,
        observed_transfers: fingerprint.flows.total_transfers,
        warnings,
    }
}
