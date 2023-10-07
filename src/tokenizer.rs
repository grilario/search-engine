use unidecode::unidecode;

#[derive(Debug)]
pub struct Tokenizer {
    pub split_tokens: Vec<char>,
}

impl Tokenizer {
    pub fn encode(&self, text: &str) -> Vec<String> {
        let tokens: Vec<String> = text
            .trim()
            .split(|c: char| c.is_whitespace() || self.split_tokens.contains(&c))
            .map(|word| unidecode(word).to_ascii_lowercase())
            .collect();

        tokens
    }
}
