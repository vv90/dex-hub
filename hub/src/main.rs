use anyhow::Result;

mod graph;
mod token;

#[tokio::main]
async fn main() -> Result<()> {
    evm::run_chains().await?;

    Ok(())
}