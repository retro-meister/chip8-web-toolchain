use crate::utils;
use TokenType::*;

use wasm_bindgen::prelude::*;

use std::array::IntoIter;
use std::collections::HashMap;
use std::fmt;
use std::iter::FromIterator;

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum TokenType {
    //literals:
    Identifier(String),
    Number(u16),

    //keywords:
    True,
    False,
    If,
    Else,
    And,
    Or,
    Var,
    While,
    Not,
    Fn,

    //in-built global CHIP-8 variables
    DT,
    ST,
    I,

    //in-built functions
    Rand,
    Draw,
    Key,

    //single-char tokens:
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Plus,
    Minus,
    ForwardSlash,
    Semicolon,
    Equals,
    Comma,

    //two-char tokens:
    EqualsEquals,
    NotEquals,

    EndOfFile,
    ErrorToken,
}

#[derive(Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub line: u32,
}

impl Token {
    pub fn new(token_type: TokenType, line: u32) -> Token {
        Token { token_type, line }
    }

    pub fn token_type(&self) -> TokenType {
        self.token_type.clone()
    }

    pub fn line(&self) -> u32 {
        self.line
    }
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[wasm_bindgen]
pub struct Lexer {
    src: Vec<char>,
    start: usize,
    current: usize,
    line: u32,
    tokens: Vec<Token>,
    keywords: HashMap<String, TokenType>,
}

#[wasm_bindgen]
impl Lexer {
    pub fn new(src: &str) -> Lexer {
        Lexer {
            src: src.chars().collect(),
            start: 0,
            current: 0,
            line: 0,
            tokens: Vec::new(),
            keywords: HashMap::<_, _>::from_iter(IntoIter::new([
                (String::from("true"), True),
                (String::from("false"), False),
                (String::from("if"), If),
                (String::from("else"), Else),
                (String::from("and"), And),
                (String::from("or"), Or),
                (String::from("var"), Var),
                (String::from("while"), While),
                (String::from("fn"), Fn),
                (String::from("DT"), DT),
                (String::from("ST"), ST),
                (String::from("I"), I),
                (String::from("RAND"), Rand),
                (String::from("DRAW"), Draw),
                (String::from("KEY"), Key),
            ])),
        }
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.peek() != expected {
            return false;
        }
        self.advance();
        true
    }

    fn peek(&mut self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        self.src[self.current]
    }

    fn advance(&mut self) -> char {
        let ret = self.peek();
        self.current += 1;
        ret
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.src.len()
    }

    pub fn lex(&mut self) {
        while !self.is_at_end() {
            self.start = self.current;

            let character = self.advance();
            match character {
                '+' => self.tokens.push(Token::new(Plus, self.line)),
                '-' => self.tokens.push(Token::new(Minus, self.line)),
                '/' => self.tokens.push(Token::new(ForwardSlash, self.line)),
                '{' => self.tokens.push(Token::new(LeftBrace, self.line)),
                '}' => self.tokens.push(Token::new(RightBrace, self.line)),
                '(' => self.tokens.push(Token::new(LeftParen, self.line)),
                ')' => self.tokens.push(Token::new(RightParen, self.line)),
                ';' => self.tokens.push(Token::new(Semicolon, self.line)),
                ',' => self.tokens.push(Token::new(Comma, self.line)),
                '=' => match self.match_char('=') {
                    true => self.tokens.push(Token::new(EqualsEquals, self.line)),
                    false => self.tokens.push(Token::new(Equals, self.line)),
                },
                '!' => match self.match_char('=') {
                    true => self.tokens.push(Token::new(NotEquals, self.line)),
                    false => self.tokens.push(Token::new(Not, self.line)),
                },
                '\n' => self.line += 1,
                _ => {
                    if character.is_digit(10) {
                        while self.peek().is_digit(10) {
                            self.advance();
                        }
                        self.tokens.push(Token::new(
                            Number(
                                self.src[self.start..self.current]
                                    .iter()
                                    .collect::<String>()
                                    .parse()
                                    .unwrap(),
                            ),
                            self.line,
                        ));
                    } else if character.is_alphabetic() {
                        while self.peek().is_alphanumeric() {
                            self.advance();
                        }

                        let ident = self.src[self.start..self.current]
                            .iter()
                            .collect::<String>();

                        match self.keywords.get(&ident) {
                            None => self.tokens.push(Token::new(Identifier(ident), self.line)),
                            Some(x) => self.tokens.push(Token::new(x.clone(), self.line)),
                        }
                    } else if character.is_whitespace() {
                        ()
                    } else {
                        self.tokens.push(Token::new(ErrorToken, self.line));
                    }
                }
            }
        }
        self.tokens.push(Token::new(EndOfFile, self.line));
    }

    pub fn stringify_tokens(&self) -> String {
        self.tokens
            .iter()
            .map(|t| t.token_type.to_string())
            .collect::<Vec<String>>()
            .join(" ")
    }
}

impl Lexer {
    pub fn tokens(&self) -> &Vec<Token> {
        &self.tokens
    }
}

#[cfg(test)]
mod tests {
    use super::Lexer;
    use super::*;
    //use super::Token::*;

    #[test]
    pub fn test_is_at_end() {
        let mut l = Lexer::new("test test 123 55");
        assert_eq!(false, l.is_at_end());
        l.current = l.src.len();
        assert_eq!(true, l.is_at_end());
    }

    #[test]
    pub fn test_lex() {
        let mut l = Lexer::new(
            "( 123 
            55 testident var else asdfg",
        );
        l.lex();
        assert!(utils::vectors_equivalent(
            l.tokens.iter().map(|t| t.clone().token_type).collect(),
            vec![
                LeftParen,
                Number(123),
                Number(55),
                Identifier(String::from("testident")),
                Var,
                Else,
                Identifier(String::from("asdfg")),
                EndOfFile
            ]
        ));
        assert!(l.line == 1);

        let mut l = Lexer::new(
            "
        var a = 50; 
        a = a + 20;",
        );
        l.lex();
        assert!(utils::vectors_equivalent(
            l.tokens.iter().map(|t| t.clone().token_type).collect(),
            vec![
                Var,
                Identifier(String::from("a")),
                Equals,
                Number(50),
                Semicolon,
                Identifier(String::from("a")),
                Equals,
                Identifier(String::from("a")),
                Plus,
                Number(20),
                Semicolon,
                EndOfFile
            ]
        ));

        assert!(l.line == 2);
    }

    #[test]
    pub fn test_two_characters() {
        let mut l = Lexer::new("8 == 5 != !0;");
        l.lex();
        assert_eq!(
            l.stringify_tokens(),
            String::from(
                "Number(8) EqualsEquals Number(5) NotEquals Not Number(0) Semicolon EndOfFile"
            )
        );
    }

    #[test]
    pub fn test_stringify_tokens() {
        let mut l = Lexer::new("test test 123 55");
        l.lex();
        assert_eq!(
            l.stringify_tokens(),
            String::from(
                "Identifier(\"test\") Identifier(\"test\") Number(123) Number(55) EndOfFile"
            )
        );
    }

    #[test]
    pub fn test_globals() {
        let mut l = Lexer::new("ST test test DT 123 I 55 RAND");
        l.lex();
        assert_eq!(
            l.stringify_tokens(),
            String::from(
                "ST Identifier(\"test\") Identifier(\"test\") DT Number(123) I Number(55) Rand EndOfFile"
            )
        );
    }

    #[test]
    pub fn test_keywords() {
        let mut l = Lexer::new("ST test test DT var while 55 RAND");
        l.lex();
        assert_eq!(
            l.stringify_tokens(),
            String::from(
                "ST Identifier(\"test\") Identifier(\"test\") DT Var While Number(55) Rand EndOfFile"
            )
        );
    }
}
