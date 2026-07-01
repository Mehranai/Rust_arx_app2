use std::sync::Arc;

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use clickhouse::Client;
use serde::Deserialize;

use crate::config::AppConfig;
use crate::services::tron::wallet_aml_risk::{
    WalletAmlRiskAssessment, build_wallet_aml_risk_assessment,
};
use crate::utils::tron_address::normalize_tron_address;

#[derive(Debug, Deserialize)]
pub struct WalletAmlRiskQuery {
    pub window_days: Option<u16>,
    pub top_counterparties: Option<usize>,
    pub max_events: Option<u64>,
}

#[derive(Debug)]
pub struct WalletAmlRiskError {
    status: StatusCode,
    message: String,
}

impl WalletAmlRiskError {
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

impl IntoResponse for WalletAmlRiskError {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}

pub async fn tron_wallet_aml_risk(
    Path(address): Path<String>,
    Query(params): Query<WalletAmlRiskQuery>,
) -> Result<Json<WalletAmlRiskAssessment>, WalletAmlRiskError> {
    let config = AppConfig::from_env();
    let address = normalize_tron_address(&address).ok_or_else(|| {
        WalletAmlRiskError::bad_request(format!("invalid Tron wallet address: {address}"))
    })?;

    let clickhouse = Arc::new(
        Client::default()
            .with_url(&config.clickhouse_url)
            .with_user(&config.clickhouse_user)
            .with_password(&config.clickhouse_pass)
            .with_database(&config.clickhouse_db_tron),
    );

    let assessment = build_wallet_aml_risk_assessment(
        clickhouse,
        &address,
        params.window_days,
        params.top_counterparties,
        params.max_events,
    )
    .await
    .map_err(WalletAmlRiskError::internal)?;

    Ok(Json(assessment))
}
