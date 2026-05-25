use anyhow::Result;
use arz_axum_for_services::router::build_router;

#[tokio::main]
async fn main() -> Result<()> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    println!("TRON graph API listening on http://0.0.0.0:3000");

    axum::serve(listener, build_router()).await?;

    Ok(())
}
