mod api;
mod core;
mod fs;
mod models;

use anyhow::Result;
use std::io::Read;

#[tokio::main]
async fn main() -> Result<()> {
    let mut stdin_data = String::new();
    std::io::stdin().read_to_string(&mut stdin_data)?;

    let payload: models::ActionPayload = serde_json::from_str(&stdin_data)?;

    core::execute_collect(&payload).await?;

    Ok(())
}
