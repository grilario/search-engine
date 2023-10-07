use std::fmt;

use indexmap::IndexMap;
use postcard::{from_bytes, to_allocvec};
use rocksdb::{DBCompressionType, IteratorMode, Options, DB};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{cosine_similarity, tokenizer::Tokenizer, Result};

#[derive(Clone, Serialize, Deserialize)]
pub struct Document {
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
    database: DB,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let path = "storage";
        let mut opts = Options::default();

        opts.set_compression_type(DBCompressionType::Snappy);
        opts.create_if_missing(true);

        Ok(Storage {
            tokenizer: Tokenizer {
                split_tokens: vec!['-', '(', ')', '_', '.', ',', '/', '\\', '[', ']', '{', '}'],
            },
            database: DB::open(&opts, path)?,
        })
    }

    pub fn insert(&mut self, title: String, description: String, texts: Vec<String>) {
        let mut texts_tokenized: Vec<Vec<String>> = Vec::new();

        for text in texts {
            let tokens = self.tokenizer.encode(&text);
            texts_tokenized.push(tokens);
        }

        let document_as_bytes: Vec<u8> = to_allocvec(&Document {
            title,
            description,
            texts_tokenized,
        })
        .unwrap();

        self.database
            .put(Uuid::new_v4(), document_as_bytes)
            .unwrap();
    }

    pub fn search(&self, query: String, limit: usize) -> Vec<Document> {
        #[derive(Debug)]
        struct SearchResult {
            document: Document,
            similarity: f32,
        }

        let mut results = Vec::new();

        let query = self.tokenizer.encode(&query);

        'documents: for row in self.database.iterator(IteratorMode::Start) {
            let document: Document = from_bytes(&row.unwrap().1).unwrap();

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

        results
            .iter()
            .take(limit)
            .map(|result| result.document.clone())
            .rev()
            .collect()
    }
}
