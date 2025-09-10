use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    evm::run_chains().await?;

    Ok(())
}
