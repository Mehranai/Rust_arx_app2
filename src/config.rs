#[derive(Debug, Clone)]
pub enum AppMode {
    Eth,
    Btc,
    Bsc,
    Tron,
}

#[derive(Debug, Clone)]
pub enum SyncMode {
    Backfill,
    Live,
    Auto,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub mode: AppMode,
    pub sync_mode: SyncMode,

    pub clickhouse_url: String,
    pub clickhouse_user: String,
    pub clickhouse_pass: String,

    pub clickhouse_db_eth: String,
    pub clickhouse_db_btc: String,
    pub clickhouse_db_bsc: String,
    pub clickhouse_db_tron: String,

    pub eth_rpc_url: Option<String>,
    pub bsc_rpc_url: Option<String>,
    pub btc_api_url: Option<String>,
    pub tron_rpc_url: Option<String>,
    pub tron_api_key: Option<String>,
    //pub btc_api_url: String,
    pub btc_start_block: u64,
    pub eth_start_block: u64,
    pub bsc_start_block: u64,
    pub tron_start_block: u64,

    pub total_btc_txs: u64,
    pub total_eth_txs: u64,
    pub total_bsc_txs: u64,
    pub total_tron_txs: u64,

    // rate limit
    pub rpc_timeout_seconds: u64,
    pub rpc_max_concurrency: usize,

    pub tx_worker_concurrency: usize,

    pub neo4j_uri: String,
    pub neo4j_username: String,
    pub neo4j_password: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let mode = AppMode::Tron;
        let sync_mode: SyncMode = SyncMode::Backfill;
        Self {
            mode,
            sync_mode,
            clickhouse_url: "http://localhost:8123".into(),
            clickhouse_user: "admin".into(),
            clickhouse_pass: "mehran.admin".into(),

            clickhouse_db_eth:"eth_db".into(),
            clickhouse_db_btc:"btc_db".into(),
            clickhouse_db_bsc:"bsc_db".into(),
            clickhouse_db_tron:"tron_db".into(),

            //eth_rpc_url: Some("http://localhost:8545".into()),
            eth_rpc_url: Some("https://rpc.ankr.com/eth/a4ce905377a7aa94ded62bf6efb50b20acde76159d163f8de77a16ec6237137b".into()),
            btc_api_url: Some("https://blockstream.info/api".into()),
            bsc_rpc_url: Some("https://rpc.ankr.com/bsc/a4ce905377a7aa94ded62bf6efb50b20acde76159d163f8de77a16ec6237137b".into()),
            tron_rpc_url: Some("https://api.trongrid.io".into()),
            tron_api_key : Some("c1e0a149-a5c9-4d3f-aa64-c08053e16be7".into()),

            btc_start_block: 831000,
            eth_start_block: 90000,
            bsc_start_block: 15000000,
            tron_start_block: 55100000,

            total_btc_txs: 500,
            total_eth_txs: 500,
            total_bsc_txs: 500,
            total_tron_txs: 200,

            rpc_timeout_seconds: 120,
            rpc_max_concurrency: 2,

            tx_worker_concurrency: 2,

            neo4j_uri: "localhost:7687".into(),
            neo4j_username: "neo4j".into(),
            neo4j_password: "password".into(),
        }
    }
}
