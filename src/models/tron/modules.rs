use clickhouse::Row;
use serde::Serialize;

//
// ======================================================
// TRANSACTIONS
// ======================================================
//

#[derive(Debug, Row, Serialize, Clone)]
pub struct TransactionRow {
    pub tx_hash: String,

    pub block_number: u64,

    // Tron raw timestamp in milliseconds.
    pub timestamp: u64,

    pub from_address: String,
    pub to_address: String,

    pub contract_address: String,

    pub contract_type: String,

    pub amount: u128,

    pub fee: u128,

    pub energy_fee: u128,

    pub net_fee: u128,

    pub energy_usage: u64,

    pub energy_usage_total: u64,

    pub net_usage: u64,

    pub status: u8,

    pub memo: String,
}

//
// ======================================================
// TOKEN TRANSFERS
// ======================================================
//

#[derive(Debug, Row, Serialize, Clone)]
pub struct TronTokenTransferRow {
    pub tx_hash: String,

    pub block_number: u64,

    pub timestamp: u64,

    pub log_index: u32,

    pub token_address: String,

    pub token_symbol: String,

    pub decimals: u8,

    pub from_address: String,

    pub to_address: String,

    pub amount: u128,

    pub amount_decimal: f64,

    pub is_mint: u8,

    pub is_burn: u8,

    pub event_signature: String,
}

//
// ======================================================
// RAW LOGS
// ======================================================
//

#[derive(Debug, Row, Serialize, Clone)]
pub struct TronRawLogRow {
    pub tx_hash: String,

    pub block_number: u64,

    pub log_index: u32,

    pub contract_address: String,

    pub topics: Vec<String>,

    pub data: String,

    pub removed: u8,

    pub timestamp: u64,
}

//
// ======================================================
// CONTRACT CALLS
// ======================================================
//

#[derive(Debug, Row, Serialize, Clone)]
pub struct TronContractCallRow {
    pub tx_hash: String,

    pub block_number: u64,

    pub timestamp: u64,

    pub caller: String,

    pub contract_address: String,

    pub protocol: String,

    pub interaction_type: String,

    pub method_id: String,

    pub token_in: String,

    pub amount_in: u128,

    pub token_out: String,

    pub amount_out: u128,

    pub confidence: f32,
}

//
// ======================================================
// AML EVENTS
// ======================================================
//

#[derive(Debug, Row, Serialize, Clone)]
pub struct TronClassifiedEventRow {
    pub event_id: String,

    pub tx_hash: String,

    pub block_number: u64,

    pub timestamp: u64,

    pub event_type: String,

    pub protocol: String,

    pub user_address: String,

    pub counterparty: String,

    pub token_in: String,

    pub amount_in: u128,

    pub token_out: String,

    pub amount_out: u128,

    pub confidence: f32,
}

//
// ======================================================
// TRANSACTION FEATURES
// ======================================================
//

#[derive(Debug, Row, Serialize, Clone)]
pub struct TransactionFeatureRow {
    pub tx_hash: String,

    pub block_number: u64,

    pub timestamp: u64,

    pub transaction_type: String,

    pub transaction_subtype: String,

    pub classification_confidence: f32,

    pub classification_source: String,

    pub protocol: String,

    pub method_id: String,

    pub is_swap: u8,

    pub is_bridge: u8,

    pub is_mint: u8,

    pub is_burn: u8,

    pub is_liquidity_add: u8,

    pub is_liquidity_remove: u8,

    pub is_contract_call: u8,

    pub unique_tokens: u16,

    pub participants: u16,

    pub hop_count: u16,

    pub fan_in: u16,

    pub fan_out: u16,
}

//
// ======================================================
// TRANSACTION RISK
// ======================================================
//

#[derive(Debug, Row, Serialize, Clone)]
pub struct TransactionRiskRow {
    pub tx_hash: String,

    pub block_number: u64,

    pub timestamp: u64,

    pub risk_score: u8,

    pub risk_level: String,

    pub transaction_type: String,

    pub transaction_subtype: String,

    pub is_swap: u8,

    pub is_bridge: u8,

    pub is_contract_call: u8,

    pub unique_tokens: u16,

    pub participants: u16,

    pub risk_reasons: Vec<String>,

    pub exposure_depth: u16,

    pub touches_sanctioned: u8,

    pub touches_mixer: u8,

    pub touches_exchange: u8,
}

//
// ======================================================
// ADDRESS ENERGY
// ======================================================
//

#[derive(Debug, Row, Serialize, Clone)]
pub struct TronAddressEnergyRow {
    pub address: String,

    pub block_number: u64,

    pub energy_usage: u64,

    pub energy_fee: u64,

    pub net_usage: u64,

    pub net_fee: u64,

    pub tx_hash: String,

    pub timestamp: u64,
}
