// 基本上把tantivy-jieba照搬了过来
// 注释掉的实现是套两层，不过我觉得tantivy-jieba本身就不是很复杂，直接搬过来还好操作一点

use tantivy::tokenizer::*;

use once_cell::sync::Lazy;
static JIEBA: Lazy<jieba_rs::Jieba> = Lazy::new(|| jieba_rs::Jieba::new());
static EN_STEMMER: Lazy<rust_stemmers::Stemmer> = Lazy::new(|| rust_stemmers::Stemmer::create(rust_stemmers::Algorithm::English));
static REGEX_PUNCTUATION: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"^[\s\p{P}]*$").unwrap());
static REGEX_ENGLISH: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"^[a-zA-Z]*$").unwrap());

/* #[derive(Clone)]
pub struct MixedTokenizer {
    chinese_tokenizer: TextAnalyzer,
}

impl MixedTokenizer {
    pub fn new() -> Self {
        let chinese_tokenizer = TextAnalyzer::from(tantivy_jieba::JiebaTokenizer {});

        MixedTokenizer { chinese_tokenizer }
    }
}

impl Tokenizer for MixedTokenizer {
    type TokenStream<'a> = MixedTokenStream<'a>;
    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        let token_stream = self.chinese_tokenizer.token_stream(text);
        MixedTokenStream {
            token_stream,
            re: regex::Regex::new(r"^[\s\p{P}]*$").unwrap(),
        }
    }
}

pub struct MixedTokenStream<'a> {
    token_stream: BoxTokenStream<'a>,
    re: regex::Regex,
}

impl TokenStream for MixedTokenStream<'_> {
    fn advance(&mut self) -> bool {
        while self.token_stream.advance() {
            let token = self.token_stream.token();
            if !self.re.is_match(&token.text) {
                return true;
            }
        }
        false
    }

    fn token(&self) -> &Token {
        self.token_stream.token()
    }

    fn token_mut(&mut self) -> &mut Token {
        self.token_stream.token_mut()
    }
}
 */

#[derive(Clone)]
pub struct MixedTokenizer;

impl MixedTokenizer {
    pub fn new() -> Self {
        MixedTokenizer {}
    }
}

// 中文原样输出，英文stem再加3-gram
impl Tokenizer for MixedTokenizer {
    type TokenStream<'a> = MixedTokenStream;

    fn token_stream(&mut self, text: &str) -> MixedTokenStream {
        let mut indices = text.char_indices().collect::<Vec<_>>();
        indices.push((text.len(), '\0'));
        let mut tokens = Vec::new();
        let orig_tokens = JIEBA.tokenize(text, jieba_rs::TokenizeMode::Search, true);
        for token in orig_tokens {
            let text = String::from(&text[(indices[token.start].0)..(indices[token.end].0)]);
            if REGEX_PUNCTUATION.is_match(&text) {
                continue;
            } else if REGEX_ENGLISH.is_match(&text) {
                if text.len() > 40 {
                    continue;
                }
                let stem = EN_STEMMER.stem(&text);
                tokens.push(Token {
                    offset_from: indices[token.start].0,
                    offset_to: indices[token.end].0,
                    position: token.start,
                    text: stem.to_string(),
                    position_length: token.end - token.start,
                });
                if stem.len() > 3 {
                    for i in 0..stem.len() - 2 {
                        tokens.push(Token {
                            offset_from: indices[token.start].0 + i,
                            offset_to: indices[token.start].0 + i + 3,
                            position: token.start + i,
                            text: String::from(&stem[i..i + 3]),
                            position_length: 3,
                        });
                    }
                }
            } else {
                tokens.push(Token {
                    offset_from: token.start,
                    offset_to: indices[token.end].0,
                    position: token.start,
                    text,
                    position_length: token.end - token.start,
                });
            }
        }
        MixedTokenStream { tokens, index: 0 }
    }
}


pub struct MixedTokenStream {
    tokens: Vec<Token>,
    index: usize,
}

impl TokenStream for MixedTokenStream {
    fn advance(&mut self) -> bool {
        if self.index < self.tokens.len() {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn token(&self) -> &Token {
        &self.tokens[self.index - 1]
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.tokens[self.index - 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mixed_tokenizer() {
        let mut tokenizer = MixedTokenizer::new();
        let mut token_stream = tokenizer.token_stream("我正在testing这个mixed语言, a test for mixed language. a tests for stemmer.");
        let mut tokens = Vec::new();
        while token_stream.advance() {
            tokens.push(token_stream.token().text.clone());
        }
        println!("{:?}", tokens);
    }
}