use crate::db::init::run_sql;
use anyhow::Context;
use clickhouse::Client;
use serde::Deserialize;

const TRON_DB: &str = "tron_db";

pub async fn init_tron_db(client: &Client) -> anyhow::Result<()> {
    client
        .query("CREATE DATABASE IF NOT EXISTS tron_db")
        .execute()
        .await
        .context("failed to create tron_db")?;

    drop_legacy_tables(client).await?;
    drop_legacy_materialized_views(client).await?;
    drop_obsolete_tron_tables(client).await?;
    drop_rebuildable_wallet_asset_balance_objects(client).await?;
    drop_incompatible_tables(client).await?;

    let wallet_asset_balance_deltas_existed =
        table_exists(client, "wallet_asset_balance_deltas").await?;

    let sql = include_str!("../../sql/init_database_tron.sql");
    run_sql(client, sql).await?;

    if !wallet_asset_balance_deltas_existed
        || table_is_empty(client, "wallet_asset_balance_deltas").await?
    {
        backfill_wallet_asset_balance_deltas(client).await?;
    }

    Ok(())
}

async fn drop_legacy_tables(client: &Client) -> anyhow::Result<()> {
    let tables = client
        .query(
            r#"
            SELECT name
            FROM system.tables
            WHERE database = ?
              AND position(name, '_legacy_') > 0
            "#,
        )
        .bind(TRON_DB)
        .fetch_all::<TableInfo>()
        .await
        .context("failed to inspect legacy Tron tables")?;

    for table in tables {
        let stmt = format!("DROP TABLE IF EXISTS {}.{}", TRON_DB, table.name);

        eprintln!(
            "[TRON SCHEMA] Dropping legacy ClickHouse table {}.{}",
            TRON_DB, table.name
        );

        client
            .query(&stmt)
            .execute()
            .await
            .with_context(|| format!("failed to drop legacy table {}", table.name))?;
    }

    Ok(())
}

async fn drop_obsolete_tron_tables(client: &Client) -> anyhow::Result<()> {
    for table in obsolete_tron_tables() {
        if table_exists(client, table).await? {
            let stmt = format!("DROP TABLE IF EXISTS {}.{}", TRON_DB, table);

            eprintln!(
                "[TRON SCHEMA] Dropping obsolete ClickHouse table {}.{}",
                TRON_DB, table
            );

            client
                .query(&stmt)
                .execute()
                .await
                .with_context(|| format!("failed to drop obsolete table {}", table))?;
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize, clickhouse::Row)]
struct ColumnInfo {
    name: String,
    #[serde(rename = "type")]
    data_type: String,
}

#[derive(Debug)]
struct TableSchema {
    table: &'static str,
    columns: &'static [(&'static str, &'static str)],
}

async fn drop_incompatible_tables(client: &Client) -> anyhow::Result<()> {
    let mut incompatible_tables = Vec::new();

    for schema in required_tron_schemas() {
        let columns = load_columns(client, schema.table).await?;

        if columns.is_empty() {
            continue;
        }

        if !schema_matches(&columns, schema.columns) {
            incompatible_tables.push(schema.table);
        }
    }

    if incompatible_tables.is_empty() {
        return Ok(());
    }

    for table in incompatible_tables {
        drop_table(client, table).await?;
    }

    Ok(())
}

async fn load_columns(client: &Client, table: &str) -> anyhow::Result<Vec<ColumnInfo>> {
    client
        .query(
            r#"
            SELECT
                name,
                type
            FROM system.columns
            WHERE database = ?
              AND table = ?
            "#,
        )
        .bind(TRON_DB)
        .bind(table)
        .fetch_all::<ColumnInfo>()
        .await
        .with_context(|| format!("failed to inspect ClickHouse schema for {}", table))
}

fn schema_matches(actual: &[ColumnInfo], required: &[(&str, &str)]) -> bool {
    required.iter().all(|(required_name, required_type)| {
        actual
            .iter()
            .any(|column| column.name == *required_name && column.data_type == *required_type)
    })
}

async fn table_exists(client: &Client, table: &str) -> anyhow::Result<bool> {
    let count = client
        .query(
            r#"
            SELECT count()
            FROM system.tables
            WHERE database = ?
              AND name = ?
            "#,
        )
        .bind(TRON_DB)
        .bind(table)
        .fetch_one::<u64>()
        .await
        .with_context(|| format!("failed to inspect ClickHouse table {}", table))?;

    Ok(count > 0)
}

async fn table_is_empty(client: &Client, table: &str) -> anyhow::Result<bool> {
    let stmt = format!("SELECT count() FROM {}.{}", TRON_DB, table);

    let count = client
        .query(&stmt)
        .fetch_one::<u64>()
        .await
        .with_context(|| format!("failed to count ClickHouse table {}", table))?;

    Ok(count == 0)
}

#[derive(Debug, Deserialize, clickhouse::Row)]
struct TableInfo {
    name: String,
}

async fn drop_legacy_materialized_views(client: &Client) -> anyhow::Result<()> {
    let views = client
        .query(
            r#"
            SELECT name
            FROM system.tables
            WHERE database = ?
              AND engine = 'MaterializedView'
              AND (
                    name IN ('mv_token_delta_from', 'mv_token_delta_to')
                    OR startsWith(name, 'mv_token_delta_from_legacy_')
                    OR startsWith(name, 'mv_token_delta_to_legacy_')
                    OR startsWith(name, 'mv_token_balance_legacy_')
                  )
            "#,
        )
        .bind(TRON_DB)
        .fetch_all::<TableInfo>()
        .await
        .context("failed to inspect legacy Tron materialized views")?;

    for view in views {
        let stmt = format!("DROP TABLE IF EXISTS {}.{}", TRON_DB, view.name);

        eprintln!(
            "[TRON SCHEMA] Dropping legacy ClickHouse materialized view {}.{}",
            TRON_DB, view.name
        );

        client
            .query(&stmt)
            .execute()
            .await
            .with_context(|| format!("failed to drop legacy materialized view {}", view.name))?;
    }

    Ok(())
}

async fn drop_table(client: &Client, table: &str) -> anyhow::Result<()> {
    let stmt = format!("DROP TABLE IF EXISTS {}.{}", TRON_DB, table);

    eprintln!(
        "[TRON SCHEMA] Dropping incompatible ClickHouse table {}.{}",
        TRON_DB, table
    );

    client
        .query(&stmt)
        .execute()
        .await
        .with_context(|| format!("failed to drop incompatible table {}", table))?;

    Ok(())
}

async fn drop_rebuildable_wallet_asset_balance_objects(client: &Client) -> anyhow::Result<()> {
    let objects = [
        "mv_wallet_asset_balance_trx_from",
        "mv_wallet_asset_balance_trx_to",
        "mv_wallet_asset_balance_token_from",
        "mv_wallet_asset_balance_token_to",
        "mv_wallet_asset_delta_trx_from",
        "mv_wallet_asset_delta_trx_to",
        "mv_wallet_asset_delta_token_from",
        "mv_wallet_asset_delta_token_to",
        "wallet_asset_balances",
    ];

    for object in objects {
        if table_exists(client, object).await? {
            let stmt = format!("DROP TABLE IF EXISTS {}.{}", TRON_DB, object);

            eprintln!(
                "[TRON SCHEMA] Rebuilding derived ClickHouse object {}.{}",
                TRON_DB, object
            );

            client
                .query(&stmt)
                .execute()
                .await
                .with_context(|| format!("failed to drop derived object {}", object))?;
        }
    }

    Ok(())
}

async fn backfill_wallet_asset_balance_deltas(client: &Client) -> anyhow::Result<()> {
    let statements = [
        r#"
        INSERT INTO tron_db.wallet_asset_balance_deltas
        (
            tx_hash,
            block_number,
            timestamp,
            address,
            asset_type,
            asset_id,
            delta_raw,
            direction
        )
        SELECT
            tx_hash,
            block_number,
            timestamp,
            from_address AS address,
            'native' AS asset_type,
            'TRX' AS asset_id,
            -toInt256(amount) AS delta_raw,
            -1 AS direction
        FROM tron_db.transactions
        WHERE from_address != ''
          AND amount > 0
        "#,
        r#"
        INSERT INTO tron_db.wallet_asset_balance_deltas
        (
            tx_hash,
            block_number,
            timestamp,
            address,
            asset_type,
            asset_id,
            delta_raw,
            direction
        )
        SELECT
            tx_hash,
            block_number,
            timestamp,
            to_address AS address,
            'native' AS asset_type,
            'TRX' AS asset_id,
            toInt256(amount) AS delta_raw,
            1 AS direction
        FROM tron_db.transactions
        WHERE to_address != ''
          AND amount > 0
        "#,
        r#"
        INSERT INTO tron_db.wallet_asset_balance_deltas
        (
            tx_hash,
            block_number,
            timestamp,
            address,
            asset_type,
            asset_id,
            delta_raw,
            direction
        )
        SELECT
            tx_hash,
            block_number,
            timestamp,
            from_address AS address,
            'trc20' AS asset_type,
            token_address AS asset_id,
            -toInt256(amount) AS delta_raw,
            -1 AS direction
        FROM tron_db.token_transfers
        WHERE from_address != 'T9yD14Nj9j7xAB4dbGeiX9h8unkKHxuWwb'
          AND amount > 0
        "#,
        r#"
        INSERT INTO tron_db.wallet_asset_balance_deltas
        (
            tx_hash,
            block_number,
            timestamp,
            address,
            asset_type,
            asset_id,
            delta_raw,
            direction
        )
        SELECT
            tx_hash,
            block_number,
            timestamp,
            to_address AS address,
            'trc20' AS asset_type,
            token_address AS asset_id,
            toInt256(amount) AS delta_raw,
            1 AS direction
        FROM tron_db.token_transfers
        WHERE to_address != 'T9yD14Nj9j7xAB4dbGeiX9h8unkKHxuWwb'
          AND amount > 0
        "#,
    ];

    eprintln!("[TRON SCHEMA] Backfilling wallet_asset_balance_deltas from existing transfers");

    for statement in statements {
        client
            .query(statement)
            .execute()
            .await
            .context("failed to backfill wallet_asset_balance_deltas")?;
    }

    Ok(())
}

fn required_tron_schemas() -> &'static [TableSchema] {
    &[
        TableSchema {
            table: "transactions",
            columns: &[
                ("tx_hash", "String"),
                ("block_number", "UInt64"),
                ("timestamp", "UInt64"),
                ("from_address", "String"),
                ("to_address", "String"),
                ("contract_address", "String"),
                ("contract_type", "String"),
                ("amount", "UInt128"),
                ("status", "UInt8"),
            ],
        },
        TableSchema {
            table: "raw_logs",
            columns: &[
                ("tx_hash", "String"),
                ("block_number", "UInt64"),
                ("log_index", "UInt32"),
                ("contract_address", "String"),
                ("topics", "Array(String)"),
                ("data", "String"),
                ("removed", "UInt8"),
                ("timestamp", "UInt64"),
            ],
        },
        TableSchema {
            table: "token_transfers",
            columns: &[
                ("tx_hash", "String"),
                ("block_number", "UInt64"),
                ("timestamp", "UInt64"),
                ("log_index", "UInt32"),
                ("token_address", "String"),
                ("from_address", "String"),
                ("to_address", "String"),
                ("amount", "UInt128"),
            ],
        },
        TableSchema {
            table: "address_relationships",
            columns: &[
                ("relationship_id", "String"),
                ("from_address", "String"),
                ("to_address", "String"),
                ("token_address", "String"),
                ("tx_hash", "String"),
                ("amount", "UInt128"),
                ("transfer_type", "String"),
            ],
        },
        TableSchema {
            table: "transaction_features",
            columns: &[
                ("tx_hash", "String"),
                ("timestamp", "UInt64"),
                ("is_swap", "UInt8"),
                ("is_contract_call", "UInt8"),
            ],
        },
        TableSchema {
            table: "transaction_risk",
            columns: &[
                ("tx_hash", "String"),
                ("timestamp", "UInt64"),
                ("risk_score", "UInt8"),
                ("risk_level", "String"),
            ],
        },
        TableSchema {
            table: "contract_metadata",
            columns: &[
                ("contract_address", "String"),
                ("contract_type", "String"),
                ("creator_address", "String"),
                ("created_block", "UInt64"),
            ],
        },
        TableSchema {
            table: "address_entity",
            columns: &[
                ("address", "String"),
                ("entity_id", "String"),
                ("entity_name", "String"),
                ("entity_type", "String"),
                ("confidence", "Float32"),
                ("source", "String"),
            ],
        },
        TableSchema {
            table: "exchange_entities",
            columns: &[
                ("entity_id", "String"),
                ("exchange_name", "String"),
                ("exchange_type", "String"),
                ("confidence", "Float32"),
            ],
        },
        TableSchema {
            table: "exchange_addresses",
            columns: &[
                ("address", "String"),
                ("entity_id", "String"),
                ("exchange_name", "String"),
                ("address_role", "String"),
                ("confidence", "Float32"),
            ],
        },
        TableSchema {
            table: "exchange_deposit_addresses",
            columns: &[
                ("address", "String"),
                ("exchange_name", "String"),
                ("hot_wallet", "String"),
                ("confidence", "Float32"),
            ],
        },
        TableSchema {
            table: "exchange_clusters",
            columns: &[
                ("cluster_id", "String"),
                ("exchange_name", "String"),
                ("address", "String"),
                ("role", "String"),
                ("confidence", "Float32"),
            ],
        },
        TableSchema {
            table: "exchange_flows",
            columns: &[
                ("tx_hash", "String"),
                ("block_number", "UInt64"),
                ("from_address", "String"),
                ("to_address", "String"),
                ("exchange_name", "String"),
                ("amount", "UInt128"),
            ],
        },
        TableSchema {
            table: "address_profiles",
            columns: &[
                ("address", "String"),
                ("total_in_tx", "UInt64"),
                ("total_out_tx", "UInt64"),
                ("total_volume_in", "UInt128"),
                ("risk_score", "Float32"),
            ],
        },
        TableSchema {
            table: "address_counterparties",
            columns: &[
                ("address", "String"),
                ("counterparty", "String"),
                ("direction", "String"),
                ("token_address", "String"),
                ("total_volume", "UInt128"),
            ],
        },
        TableSchema {
            table: "address_token_delta",
            columns: &[
                ("tx_hash", "String"),
                ("block_number", "UInt64"),
                ("timestamp", "UInt64"),
                ("address", "String"),
                ("token_address", "String"),
                ("delta", "Int128"),
                ("direction", "Int8"),
            ],
        },
        TableSchema {
            table: "address_token_balance",
            columns: &[
                ("address", "String"),
                ("token_address", "String"),
                ("balance", "Int128"),
            ],
        },
        TableSchema {
            table: "wallet_asset_balance_deltas",
            columns: &[
                ("tx_hash", "String"),
                ("block_number", "UInt64"),
                ("timestamp", "UInt64"),
                ("address", "String"),
                ("asset_type", "String"),
                ("asset_id", "String"),
                ("delta_raw", "Int256"),
                ("direction", "Int8"),
            ],
        },
        TableSchema {
            table: "wallet_asset_balances",
            columns: &[
                ("address", "String"),
                ("asset_type", "String"),
                ("asset_id", "String"),
                ("asset_symbol", "String"),
                ("asset_name", "String"),
                ("decimals", "UInt8"),
                ("balance_raw", "Int256"),
                ("balance_decimal", "Float64"),
            ],
        },
        TableSchema {
            table: "sync_state",
            columns: &[("chain", "String"), ("last_synced_block", "UInt64")],
        },
    ]
}

fn obsolete_tron_tables() -> &'static [&'static str] {
    &[
        "wallet_info",
        "owner_info",
        "contract_calls",
        "address_energy_usage",
        "wallet_state",
    ]
}
