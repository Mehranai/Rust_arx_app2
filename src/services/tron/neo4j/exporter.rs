use super::client::Neo4jClient;
use super::nodes::upsert_wallet;

use super::edges::create_transfer_edge;

use crate::models::tron::relationship::AddressRelationshipRow;

pub async fn export_relationship(
    neo4j: &Neo4jClient,
    row: &AddressRelationshipRow,
) -> anyhow::Result<()> {
    upsert_wallet(neo4j, &row.from_address).await?;

    upsert_wallet(neo4j, &row.to_address).await?;

    create_transfer_edge(
        neo4j,
        &row.from_address,
        &row.to_address,
        &row.tx_hash,
        &row.token_address,
        &row.amount.to_string(),
        row.risk_score,
        &row.transfer_type,
    )
    .await?;

    Ok(())
}
