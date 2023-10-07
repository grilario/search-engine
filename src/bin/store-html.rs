use std::fs;
use std::io::Read;

use search_engine::parser::parse_document;
use search_engine::storage::Storage;
use search_engine::Result;

#[tokio::main]
pub async fn main() -> Result<()> {
    let mut html_files = Vec::new();

    if let Ok(entries) = fs::read_dir("html") {
        for entry in entries {
            if let Ok(entry) = entry {
                let mut file = fs::File::open(entry.path()).unwrap();
                let mut buffer = String::new();
                file.read_to_string(&mut buffer).unwrap();
                html_files.push(buffer);
            }
        }
    }

    let mut storage = Storage::new()?;

    for file in html_files {
        let (title, description, texts) = parse_document(file).await?;

        storage.insert(title, description, texts);
    }

    Ok(())
}
