use std::sync::Arc;

use search_engine::{server::make_routes, storage::Storage};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let storage = Storage::new()?;
    let state = Arc::new(storage);

    axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(make_routes(state))
        .await?;

    Ok(())
}
