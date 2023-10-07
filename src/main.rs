use std::io;

use search_engine::{storage::Storage, Result};
use tokio::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    let ref storage = Storage::new()?;

    let stdin = io::stdin();

    loop {
        let mut query = String::new();

        stdin.read_line(&mut query)?;

        let now = Instant::now();

        let search_result = storage.search(query, 100);

        println!("{:#?}", search_result);

        println!("ms:{}", now.elapsed().as_millis());
    }

    Ok(())
}
