use neo4rs::query;

use super::client::Neo4jClient;
use super::queries::address_graph_query;

pub async fn get_wallet_graph(
    neo4j: &Neo4jClient,
    address: &str,
    depth: u32,
) -> anyhow::Result<()> {
    let q = query(&address_graph_query(depth)).param("address", address);

    let mut result = neo4j
        .graph
        .execute(q)
        .await
        .expect("failed to execute neo4j graph");

    while let Ok(Some(row)) = result.next().await {
        println!("{:?}", row);
    }

    Ok(())
}
