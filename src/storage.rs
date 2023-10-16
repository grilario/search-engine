use std::fmt;

use indexmap::IndexMap;
use postcard::{from_bytes, to_allocvec};
use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Serialize};

use crate::{cosine_similarity, tokenizer::Tokenizer, Result};

#[derive(Clone, Serialize, Deserialize)]
pub struct Document {
    pub url: String,
    pub title: String,
    pub description: String,
    texts_tokenized: Vec<Vec<String>>,
}

impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Document")
            .field("title", &self.title)
            .field("description", &self.description)
            .finish()
    }
}

#[derive(Debug)]
pub struct Storage {
    tokenizer: Tokenizer,
    database: r2d2::Pool<SqliteConnectionManager>,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let tokenizer = Tokenizer {
            split_tokens: vec!['-', '(', ')', '_', '.', ',', '/', '\\', '[', ']', '{', '}'],
        };
        let manager = SqliteConnectionManager::file("storage.db");
        let pool = r2d2::Pool::new(manager)?;

        pool.get()?.execute(
            "CREATE TABLE IF NOT EXISTS documents (
                url TEXT PRIMARY KEY, 
                title TEXT NOT NULL, 
                description TEXT,
                texts BLOB
            )",
            (),
        )?;

        Ok(Storage {
            tokenizer,
            database: pool,
        })
    }

    pub async fn insert(
        &mut self,
        url: String,
        title: String,
        description: String,
        texts: Vec<String>,
    ) -> anyhow::Result<()> {
        let mut texts_tokenized: Vec<Vec<String>> = Vec::new();

        for text in texts {
            let tokens = self.tokenizer.encode(&text);
            texts_tokenized.push(tokens);
        }

        let texts_as_bytes: Vec<u8> = to_allocvec(&texts_tokenized)?;

        let conn = self.database.get()?;

        conn.execute(
            "INSERT INTO documents (url, title, description, texts) VALUES (?1, ?2, ?3, ?4)",
            (&url, &title, &description, &texts_as_bytes),
        )?;

        Ok(())
    }

    pub async fn search(&self, query: String, limit: usize) -> anyhow::Result<Vec<Document>> {
        #[derive(Debug)]
        struct SearchResult {
            document: Document,
            similarity: f32,
        }

        let mut results = Vec::new();

        let query = self.tokenizer.encode(&query);

        let conn = self.database.get()?;
        let mut stmt = conn.prepare("SELECT url, title, description, texts FROM documents")?;
        let rows = stmt.query_map([], |row| {
            let texts_as_bytes: Vec<u8> = row.get(3)?;

            Ok(Document {
                url: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                texts_tokenized: from_bytes(&texts_as_bytes).map_err(|_| {
                    rusqlite::Error::InvalidColumnType(
                        3,
                        "Vec".to_owned(),
                        rusqlite::types::Type::Blob,
                    )
                })?,
            })
        })?;

        'documents: for row in rows {
            let document: Document = row?;

            for word in query.iter() {
                for text in document.texts_tokenized.iter() {
                    if text.contains(&word) {
                        results.push(SearchResult {
                            document,
                            similarity: 0.0,
                        });
                        continue 'documents;
                    }
                }
            }
        }

        for SearchResult {
            document,
            similarity,
        } in results.iter_mut()
        {
            let mut text_similarity = Vec::new();

            for text in document.texts_tokenized.iter() {
                let mut words_frequencies: IndexMap<String, (i32, i32)> = IndexMap::new();

                query.iter().for_each(|word| {
                    words_frequencies
                        .entry(word.clone())
                        .and_modify(|counter| counter.0 += 1)
                        .or_insert((1, 0));
                });

                text.iter().for_each(|word| {
                    words_frequencies
                        .entry(word.clone())
                        .and_modify(|counter| counter.1 += 1)
                        .or_insert((0, 1));
                });

                let query_frequencies: Vec<i32> = words_frequencies
                    .iter()
                    .map(|(_, frequency)| frequency.0)
                    .collect();
                let text_frequencies: Vec<i32> = words_frequencies
                    .iter()
                    .map(|(_, frequency)| frequency.1)
                    .collect();

                let cosine_similarity = cosine_similarity(&query_frequencies, &text_frequencies);

                text_similarity.push(cosine_similarity);
            }

            *similarity = text_similarity.iter().sum::<f32>() / text_similarity.len() as f32;
        }

        results.sort_by(|a, b| {
            a.similarity
                .partial_cmp(&b.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results
            .iter()
            .take(limit)
            .map(|result| result.document.clone())
            .rev()
            .collect())
    }
}
