//! Lexer: raw source text → tokens, each tagged with its source span.

use crate::span::{CompileError, Span};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenKind {
    // keywords
    Let,
    If,
    Else,
    While,
    Print,
    // literals / names
    Int,
    Ident,
    // operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,  // =
    EqEq,    // ==
    BangEq,  // !=
    Lt,      // <
    LtEq,    // <=
    Gt,      // >
    GtEq,    // >=
    // punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semi,
}

#[derive(Debug, Clone, Serialize)]
pub struct Token {
    pub kind: TokenKind,
    /// The exact source text of the token (handy for the UI).
    pub text: String,
    pub span: Span,
}

pub fn lex(src: &str) -> Result<Vec<Token>, CompileError> {
    let bytes = src.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        let c = bytes[i];

        // skip whitespace
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // skip // line comments
        if c == b'/' && bytes.get(i + 1) == Some(&b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        let start = i as u32;

        // integer literal
        if c.is_ascii_digit() {
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            push(&mut tokens, TokenKind::Int, src, start, i);
            continue;
        }

        // identifier / keyword
        if c.is_ascii_alphabetic() || c == b'_' {
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let text = &src[start as usize..i];
            let kind = match text {
                "let" => TokenKind::Let,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "while" => TokenKind::While,
                "print" => TokenKind::Print,
                _ => TokenKind::Ident,
            };
            push(&mut tokens, kind, src, start, i);
            continue;
        }

        // operators & punctuation
        let two = |b: u8| bytes.get(i + 1) == Some(&b);
        let (kind, len) = match c {
            b'+' => (TokenKind::Plus, 1),
            b'-' => (TokenKind::Minus, 1),
            b'*' => (TokenKind::Star, 1),
            b'/' => (TokenKind::Slash, 1),
            b'%' => (TokenKind::Percent, 1),
            b'=' if two(b'=') => (TokenKind::EqEq, 2),
            b'=' => (TokenKind::Assign, 1),
            b'!' if two(b'=') => (TokenKind::BangEq, 2),
            b'<' if two(b'=') => (TokenKind::LtEq, 2),
            b'<' => (TokenKind::Lt, 1),
            b'>' if two(b'=') => (TokenKind::GtEq, 2),
            b'>' => (TokenKind::Gt, 1),
            b'(' => (TokenKind::LParen, 1),
            b')' => (TokenKind::RParen, 1),
            b'{' => (TokenKind::LBrace, 1),
            b'}' => (TokenKind::RBrace, 1),
            b';' => (TokenKind::Semi, 1),
            _ => {
                return Err(CompileError::new(
                    format!("unexpected character '{}'", c as char),
                    Span::new(start, start + 1),
                ))
            }
        };
        i += len;
        push(&mut tokens, kind, src, start, i);
    }

    Ok(tokens)
}

fn push(tokens: &mut Vec<Token>, kind: TokenKind, src: &str, start: u32, end: usize) {
    tokens.push(Token {
        kind,
        text: src[start as usize..end].to_string(),
        span: Span::new(start, end as u32),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_simple_program() {
        let toks = lex("let x = 1 + 2;").unwrap();
        let kinds: Vec<TokenKind> = toks.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Let,
                TokenKind::Ident,
                TokenKind::Assign,
                TokenKind::Int,
                TokenKind::Plus,
                TokenKind::Int,
                TokenKind::Semi,
            ]
        );
        // spans point at the right text
        assert_eq!(toks[1].text, "x");
        assert_eq!(toks[1].span, Span::new(4, 5));
    }

    #[test]
    fn lexes_two_char_operators() {
        let toks = lex("== != <= >= < >").unwrap();
        let kinds: Vec<TokenKind> = toks.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::EqEq,
                TokenKind::BangEq,
                TokenKind::LtEq,
                TokenKind::GtEq,
                TokenKind::Lt,
                TokenKind::Gt,
            ]
        );
    }

    #[test]
    fn skips_comments() {
        let toks = lex("let a = 1; // the answer\nprint(a);").unwrap();
        assert_eq!(toks.len(), 10);
    }

    #[test]
    fn rejects_unknown_chars() {
        let err = lex("let x = @;").unwrap_err();
        assert!(err.message.contains("unexpected character"));
        assert_eq!(err.span.unwrap().start, 8);
    }
}
