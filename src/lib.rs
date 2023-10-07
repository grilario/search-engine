pub mod parser;
pub mod storage;
pub mod tokenizer;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn cosine_similarity(a: &[i32], b: &[i32]) -> f32 {
    let dot_product = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<i32>() as f32;
    let norm_a = (a.iter().map(|x| x * x).sum::<i32>() as f32).sqrt();
    let norm_b = (b.iter().map(|x| x * x).sum::<i32>() as f32).sqrt();

    if dot_product == 0.0 {
        return 0.0;
    };

    dot_product / (norm_a * norm_b)
}
