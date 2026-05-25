use super::client::Neo4jClient;
use neo4rs::query;

pub async fn create_transfer_edge(
    neo4j: &Neo4jClient,

    from: &str,
    to: &str,

    tx_hash: &str,

    token: &str,

    amount: &str,

    risk_score: u8,

    transfer_type: &str,
) -> anyhow::Result<()> {
    let q = query(
        "
        MERGE (a:Wallet {
            address: $from
        })

        MERGE (b:Wallet {
            address: $to
        })

        CREATE (a)-[:TRANSFER {
            tx_hash: $tx_hash,
            token: $token,
            amount: $amount,
            risk_score: $risk_score,
            transfer_type: $transfer_type
        }]->(b)
        ",
    )
    .param("from", from)
    .param("to", to)
    .param("tx_hash", tx_hash)
    .param("token", token)
    .param("amount", amount)
    .param("risk_score", risk_score as i64)
    .param("transfer_type", transfer_type);

    neo4j
        .graph
        .run(q)
        .await
        .map_err(|err| anyhow::anyhow!("failed to create Neo4j transfer edge: {:?}", err))?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn merge_transfer_edge(
    neo4j: &Neo4jClient,
    edge_id: &str,
    from: &str,
    to: &str,
    tx_hash: &str,
    token: &str,
    amount: &str,
    block_number: u64,
    timestamp: u64,
    risk_score: u8,
    transfer_type: &str,
    protocol: &str,
) -> anyhow::Result<()> {
    let q = query(
        "
        MERGE (a:Wallet { address: $from })
        SET a:TronAddress
        MERGE (b:Wallet { address: $to })
        SET b:TronAddress
        MERGE (a)-[t:TRANSFER { id: $edge_id }]->(b)
        SET t.tx_hash = $tx_hash,
            t.token = $token,
            t.amount = $amount,
            t.block_number = $block_number,
            t.timestamp = $timestamp,
            t.risk_score = $risk_score,
            t.transfer_type = $transfer_type,
            t.protocol = $protocol,
            t.chain = 'tron'
        ",
    )
    .param("from", from)
    .param("to", to)
    .param("edge_id", edge_id)
    .param("tx_hash", tx_hash)
    .param("token", token)
    .param("amount", amount)
    .param("block_number", block_number as i64)
    .param("timestamp", timestamp as i64)
    .param("risk_score", risk_score as i64)
    .param("transfer_type", transfer_type)
    .param("protocol", protocol);

    neo4j
        .graph
        .run(q)
        .await
        .map_err(|err| anyhow::anyhow!("{:?}", err))?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn merge_exchange_interaction(
    neo4j: &Neo4jClient,
    wallet_address: &str,
    exchange_name: &str,
    exchange_address: &str,
    exchange_role: &str,
    direction: &str,
    tx_hash: &str,
    token: &str,
    amount: &str,
    block_number: u64,
    confidence: f32,
) -> anyhow::Result<()> {
    let interaction_id = format!(
        "{}:{}:{}:{}:{}",
        wallet_address, exchange_name, exchange_address, tx_hash, direction
    );

    let q = query(
        "
        MERGE (w:Wallet { address: $wallet_address })
        SET w:TronAddress

        MERGE (exchange_wallet:Wallet { address: $exchange_address })
        SET exchange_wallet:TronAddress,
            exchange_wallet.node_type = 'exchange_wallet',
            exchange_wallet.exchange_name = $exchange_name,
            exchange_wallet.exchange_role = $exchange_role,
            exchange_wallet.exchange_confidence_bps = $confidence_bps

        MERGE (e:Exchange { name: $exchange_name })
        SET e.entity_type = 'exchange',
            e.chain = 'tron'

        MERGE (exchange_wallet)-[belongs:BELONGS_TO]->(e)
        SET belongs.role = $exchange_role,
            belongs.confidence_bps = $confidence_bps,
            belongs.chain = 'tron'

        MERGE (w)-[i:INTERACTED_WITH { id: $interaction_id }]->(e)
        SET i.direction = $direction,
            i.exchange_address = $exchange_address,
            i.exchange_role = $exchange_role,
            i.tx_hash = $tx_hash,
            i.token = $token,
            i.amount = $amount,
            i.block_number = $block_number,
            i.confidence_bps = $confidence_bps,
            i.chain = 'tron'
        ",
    )
    .param("wallet_address", wallet_address)
    .param("exchange_name", exchange_name)
    .param("exchange_address", exchange_address)
    .param("exchange_role", exchange_role)
    .param("direction", direction)
    .param("tx_hash", tx_hash)
    .param("token", token)
    .param("amount", amount)
    .param("block_number", block_number as i64)
    .param("confidence_bps", (confidence * 10_000.0) as i64)
    .param("interaction_id", interaction_id);

    neo4j
        .graph
        .run(q)
        .await
        .map_err(|err| anyhow::anyhow!("failed to merge Neo4j exchange interaction: {:?}", err))?;

    Ok(())
}
