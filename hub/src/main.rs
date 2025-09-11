use anyhow::Result;

use crate::pools::collect_pools;

mod graph;
mod pools;
mod tokens;

#[tokio::main]
async fn main() -> Result<()> {
    collect_pools().await?;
    Ok(())
}
