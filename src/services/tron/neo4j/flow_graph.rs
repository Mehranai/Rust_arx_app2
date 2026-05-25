use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use clickhouse::Client;
use serde::Deserialize;

use super::client::Neo4jClient;
use super::edges::{merge_exchange_interaction, merge_transfer_edge};
use super::nodes::upsert_wallet_with_metadata;
use super::types::{ExchangeFlowSummary, FlowEdge, FlowNode, Neo4jVisualization, WalletFlowGraph};

#[derive(Debug, Clone)]
struct ExchangeMetadata {
    exchange_name: String,
    exchange_role: String,
    confidence: f32,
}

#[derive(Debug, Clone, Deserialize, clickhouse::Row)]
struct RelationshipReadRow {
    relationship_id: String,
    from_address: String,
    to_address: String,
    token_address: String,
    tx_hash: String,
    block_number: u64,
    timestamp_unix: u64,
    amount_string: String,
    transfer_type: String,
    protocol: String,
    risk_score: u8,
}

#[derive(Debug, Clone, Deserialize, clickhouse::Row)]
struct ExchangeMetadataRow {
    exchange_name: String,
    address_role: String,
    confidence: f32,
    #[serde(rename = "last_seen_block")]
    _last_seen_block: u64,
}

pub async fn build_wallet_flow_graph(
    clickhouse: Arc<Client>,
    neo4j: &Neo4jClient,
    address: &str,
    depth: u8,
    per_address_limit: u64,
) -> anyhow::Result<WalletFlowGraph> {
    neo4j.ensure_schema().await?;

    let depth = depth.clamp(1, 6);
    let per_address_limit = per_address_limit.clamp(1, 2_000);

    let edges =
        load_relationship_neighborhood(clickhouse.clone(), address, depth, per_address_limit)
            .await?;

    let mut node_ids = HashSet::<String>::new();
    for edge in &edges {
        node_ids.insert(edge.from.clone());
        node_ids.insert(edge.to.clone());
    }
    node_ids.insert(address.to_string());

    let mut metadata = HashMap::<String, ExchangeMetadata>::new();
    for node_id in &node_ids {
        if let Some(exchange) = load_exchange_metadata(&clickhouse, node_id).await? {
            metadata.insert(node_id.clone(), exchange);
        }
    }

    let mut nodes = Vec::<FlowNode>::new();
    for node_id in node_ids {
        let exchange = metadata.get(&node_id);

        let node = FlowNode {
            id: node_id.clone(),
            label: exchange
                .map(|meta| format!("{} ({})", meta.exchange_name, meta.exchange_role))
                .unwrap_or_else(|| short_address(&node_id)),
            node_type: exchange
                .map(|_| "exchange_wallet".to_string())
                .unwrap_or_else(|| "wallet".to_string()),
            exchange_name: exchange.map(|meta| meta.exchange_name.clone()),
            exchange_role: exchange.map(|meta| meta.exchange_role.clone()),
            confidence: exchange.map(|meta| meta.confidence),
        };

        upsert_wallet_with_metadata(
            neo4j,
            &node.id,
            node.exchange_name.as_deref(),
            node.exchange_role.as_deref(),
            node.confidence,
        )
        .await?;

        nodes.push(node);
    }

    for edge in &edges {
        merge_transfer_edge(
            neo4j,
            &edge.id,
            &edge.from,
            &edge.to,
            &edge.tx_hash,
            &edge.token_address,
            &edge.amount,
            edge.block_number,
            edge.timestamp,
            edge.risk_score,
            &edge.transfer_type,
            &edge.protocol,
        )
        .await?;
    }

    let incoming_origins = incoming_origin_nodes(address, &nodes, &edges);

    let exchange_interactions = exchange_summaries(address, &edges, &metadata);

    for interaction in &exchange_interactions {
        merge_exchange_interaction(
            neo4j,
            address,
            &interaction.exchange_name,
            &interaction.address,
            &interaction.exchange_role,
            &interaction.direction,
            &interaction.tx_hash,
            &interaction.token_address,
            &interaction.amount,
            interaction.block_number,
            interaction.confidence,
        )
        .await?;
    }

    let neo4j_visualization = Neo4jVisualization {
        browser_url: "http://localhost:7474/browser/".to_string(),
        cypher: neo4j_browser_cypher(address, depth, per_address_limit),
        imported_wallet_nodes: nodes.len(),
        imported_transfer_edges: edges.len(),
        imported_exchange_interactions: exchange_interactions.len(),
    };

    Ok(WalletFlowGraph {
        address: address.to_string(),
        depth,
        nodes,
        edges,
        incoming_origins,
        exchange_interactions,
        neo4j: neo4j_visualization,
    })
}

async fn load_relationship_neighborhood(
    clickhouse: Arc<Client>,
    address: &str,
    depth: u8,
    per_address_limit: u64,
) -> anyhow::Result<Vec<FlowEdge>> {
    let mut queue = VecDeque::<(String, u8)>::from([(address.to_string(), 0)]);
    let mut visited = HashSet::<String>::new();
    let mut edge_ids = HashSet::<String>::new();
    let mut edges = Vec::<FlowEdge>::new();

    while let Some((current, current_depth)) = queue.pop_front() {
        if current_depth >= depth || !visited.insert(current.clone()) {
            continue;
        }

        let rows = load_relationships_for_address(&clickhouse, &current, per_address_limit).await?;

        for row in rows {
            let edge = relationship_row_to_edge(row);

            if edge_ids.insert(edge.id.clone()) {
                if current_depth + 1 < depth {
                    if edge.from != current {
                        queue.push_back((edge.from.clone(), current_depth + 1));
                    }

                    if edge.to != current {
                        queue.push_back((edge.to.clone(), current_depth + 1));
                    }
                }

                edges.push(edge);
            }
        }
    }

    Ok(edges)
}

async fn load_relationships_for_address(
    clickhouse: &Client,
    address: &str,
    limit: u64,
) -> anyhow::Result<Vec<RelationshipReadRow>> {
    let rows = clickhouse
        .query(
            r#"
            SELECT
                relationship_id,
                from_address,
                to_address,
                token_address,
                tx_hash,
                block_number,
                toUInt64(timestamp) AS timestamp_unix,
                toString(amount) AS amount_string,
                transfer_type,
                protocol,
                risk_score
            FROM address_relationships
            WHERE from_address = ? OR to_address = ?
            ORDER BY block_number DESC
            LIMIT ?
            "#,
        )
        .bind(address)
        .bind(address)
        .bind(limit)
        .fetch_all::<RelationshipReadRow>()
        .await?;

    Ok(rows)
}

async fn load_exchange_metadata(
    clickhouse: &Client,
    address: &str,
) -> anyhow::Result<Option<ExchangeMetadata>> {
    let row = clickhouse
        .query(
            r#"
            SELECT
                exchange_name,
                address_role,
                confidence,
                last_seen_block
            FROM
            (
                SELECT
                    exchange_name,
                    address_role,
                    confidence,
                    last_seen_block
                FROM exchange_addresses
                WHERE address = ?
                UNION ALL
                SELECT
                    exchange_name,
                    'DEPOSIT' AS address_role,
                    confidence,
                    last_seen_block
                FROM exchange_deposit_addresses
                WHERE address = ?
            )
            ORDER BY confidence DESC, last_seen_block DESC
            LIMIT 1
            "#,
        )
        .bind(address)
        .bind(address)
        .fetch_optional::<ExchangeMetadataRow>()
        .await?;

    Ok(row.map(|row| ExchangeMetadata {
        exchange_name: row.exchange_name,
        exchange_role: row.address_role,
        confidence: row.confidence,
    }))
}

fn relationship_row_to_edge(row: RelationshipReadRow) -> FlowEdge {
    FlowEdge {
        id: row.relationship_id,
        from: row.from_address,
        to: row.to_address,
        token_address: row.token_address,
        tx_hash: row.tx_hash,
        block_number: row.block_number,
        timestamp: row.timestamp_unix,
        amount: row.amount_string,
        transfer_type: row.transfer_type,
        protocol: row.protocol,
        risk_score: row.risk_score,
    }
}

fn incoming_origin_nodes(address: &str, nodes: &[FlowNode], edges: &[FlowEdge]) -> Vec<FlowNode> {
    let direct_senders = edges
        .iter()
        .filter(|edge| edge.to == address)
        .map(|edge| edge.from.as_str())
        .collect::<HashSet<_>>();

    nodes
        .iter()
        .filter(|node| direct_senders.contains(node.id.as_str()))
        .cloned()
        .collect()
}

fn exchange_summaries(
    address: &str,
    edges: &[FlowEdge],
    metadata: &HashMap<String, ExchangeMetadata>,
) -> Vec<ExchangeFlowSummary> {
    let mut summaries = Vec::<ExchangeFlowSummary>::new();

    for edge in edges {
        if edge.from == address {
            if let Some(exchange) = metadata.get(&edge.to) {
                summaries.push(summary_from_edge(edge, &edge.to, exchange, "outgoing"));
            }
        }

        if edge.to == address {
            if let Some(exchange) = metadata.get(&edge.from) {
                summaries.push(summary_from_edge(edge, &edge.from, exchange, "incoming"));
            }
        }
    }

    summaries
}

fn summary_from_edge(
    edge: &FlowEdge,
    exchange_address: &str,
    exchange: &ExchangeMetadata,
    direction: &str,
) -> ExchangeFlowSummary {
    ExchangeFlowSummary {
        exchange_name: exchange.exchange_name.clone(),
        exchange_role: exchange.exchange_role.clone(),
        address: exchange_address.to_string(),
        direction: direction.to_string(),
        tx_hash: edge.tx_hash.clone(),
        token_address: edge.token_address.clone(),
        amount: edge.amount.clone(),
        block_number: edge.block_number,
        confidence: exchange.confidence,
    }
}

fn short_address(address: &str) -> String {
    if address.len() <= 12 {
        return address.to_string();
    }

    format!("{}...{}", &address[..6], &address[address.len() - 4..])
}

pub fn neo4j_browser_cypher(address: &str, depth: u8, limit: u64) -> String {
    let safe_depth = depth.clamp(1, 6);
    let safe_limit = limit.clamp(1, 2_000);
    let escaped_address = address.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "MATCH p = (w:Wallet {{ address: '{}' }})-[*1..{}]-(n) RETURN p LIMIT {}",
        escaped_address, safe_depth, safe_limit
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_direct_exchange_interactions() {
        let edge = FlowEdge {
            id: "edge".to_string(),
            from: "wallet".to_string(),
            to: "exchange_wallet".to_string(),
            tx_hash: "tx".to_string(),
            token_address: "TRX".to_string(),
            amount: "100".to_string(),
            block_number: 10,
            timestamp: 1,
            transfer_type: "native_transfer".to_string(),
            protocol: "".to_string(),
            risk_score: 0,
        };

        let metadata = HashMap::from([(
            "exchange_wallet".to_string(),
            ExchangeMetadata {
                exchange_name: "Binance".to_string(),
                exchange_role: "HOT".to_string(),
                confidence: 1.0,
            },
        )]);

        let summaries = exchange_summaries("wallet", &[edge], &metadata);

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].direction, "outgoing");
        assert_eq!(summaries[0].exchange_name, "Binance");
    }

    #[test]
    fn builds_browser_cypher_for_root_wallet() {
        let cypher = neo4j_browser_cypher("TAddress", 3, 500);

        assert_eq!(
            cypher,
            "MATCH p = (w:Wallet { address: 'TAddress' })-[*1..3]-(n) RETURN p LIMIT 500"
        );
    }
}
