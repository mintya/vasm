use crate::asm::diagnostics::{Diagnostic, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Number(i64),
    String(Vec<u8>),
    Ident(String),
    LBracket,
    RBracket,
    LParen,
    RParen,
    Comma,
    Colon,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Question,
    Newline,
    Eof,
}

pub fn tokenize(source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    Lexer::new(source).lex_all()
}

struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: u32,
    col: u32,
    tokens: Vec<Token>,
    diags: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src: src.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
            tokens: Vec::new(),
            diags: Vec::new(),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        if b == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(b)
    }

    fn lex_all(mut self) -> (Vec<Token>, Vec<Diagnostic>) {
        loop {
            self.skip_blank();
            let line = self.line;
            let col = self.col;
            let start = self.pos;

            let Some(b) = self.peek() else {
                self.tokens.push(Token {
                    kind: TokenKind::Eof,
                    span: Span::new(line, col, 0),
                });
                return (self.tokens, self.diags);
            };

            let kind = match b {
                b'\n' => {
                    self.advance();
                    TokenKind::Newline
                }
                b'\r' => {
                    self.advance();
                    if self.peek() == Some(b'\n') {
                        self.advance();
                    }
                    TokenKind::Newline
                }
                b'[' => {
                    self.advance();
                    TokenKind::LBracket
                }
                b']' => {
                    self.advance();
                    TokenKind::RBracket
                }
                b'(' => {
                    self.advance();
                    TokenKind::LParen
                }
                b')' => {
                    self.advance();
                    TokenKind::RParen
                }
                b',' => {
                    self.advance();
                    TokenKind::Comma
                }
                b':' => {
                    self.advance();
                    TokenKind::Colon
                }
                b'+' => {
                    self.advance();
                    TokenKind::Plus
                }
                b'-' => {
                    self.advance();
                    TokenKind::Minus
                }
                b'*' => {
                    self.advance();
                    TokenKind::Star
                }
                b'/' => {
                    self.advance();
                    TokenKind::Slash
                }
                b'%' => {
                    self.advance();
                    TokenKind::Percent
                }
                b'?' => {
                    self.advance();
                    TokenKind::Question
                }
                b'\'' | b'"' => self.lex_string(line, col),
                b if b.is_ascii_digit() => self.lex_number(line, col),
                b if is_ident_start(b) => self.lex_ident(),
                b => {
                    self.advance();
                    self.diags.push(Diagnostic::error(
                        Span::new(line, col, 1),
                        format!("unexpected character `{}`", char::from(b).escape_default()),
                    ));
                    continue;
                }
            };

            let len = (self.pos - start) as u32;
            self.tokens.push(Token {
                kind,
                span: Span::new(line, col, len),
            });
        }
    }

    fn skip_blank(&mut self) {
        loop {
            match self.peek() {
                Some(b' ' | b'\t') => {
                    self.advance();
                }
                Some(b';') => {
                    while let Some(b) = self.peek() {
                        if b == b'\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => return,
            }
        }
    }

    fn lex_number(&mut self, line: u32, col: u32) -> TokenKind {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_alphanumeric() {
                self.advance();
            } else {
                break;
            }
        }
        let raw = &self.src[start..self.pos];

        let (digits, radix) = match raw.last().copied() {
            Some(b'h' | b'H') => (&raw[..raw.len() - 1], 16u32),
            Some(b'b' | b'B')
                if raw.len() > 1
                    && raw[..raw.len() - 1].iter().all(|&c| c == b'0' || c == b'1') =>
            {
                (&raw[..raw.len() - 1], 2u32)
            }
            _ => (raw, 10u32),
        };

        let text = std::str::from_utf8(digits).unwrap_or("");
        match i64::from_str_radix(text, radix) {
            Ok(n) => TokenKind::Number(n),
            Err(_) => {
                self.diags.push(Diagnostic::error(
                    Span::new(line, col, raw.len() as u32),
                    format!("invalid number `{}`", String::from_utf8_lossy(raw)),
                ));
                TokenKind::Number(0)
            }
        }
    }

    fn lex_ident(&mut self) -> TokenKind {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if is_ident_cont(b) {
                self.advance();
            } else {
                break;
            }
        }
        let raw = &self.src[start..self.pos];
        let text = std::str::from_utf8(raw).unwrap_or("").to_ascii_lowercase();
        TokenKind::Ident(text)
    }

    fn lex_string(&mut self, line: u32, col: u32) -> TokenKind {
        let quote = self.peek().expect("checked by caller");
        self.advance();
        let body_start = self.pos;
        loop {
            match self.peek() {
                None => {
                    self.diags.push(Diagnostic::error(
                        Span::new(line, col, (self.pos - body_start + 1) as u32),
                        "unterminated string literal",
                    ));
                    return TokenKind::String(self.src[body_start..self.pos].to_vec());
                }
                Some(b'\n') => {
                    self.diags.push(Diagnostic::error(
                        Span::new(line, col, (self.pos - body_start + 1) as u32),
                        "unterminated string literal (newline inside)",
                    ));
                    return TokenKind::String(self.src[body_start..self.pos].to_vec());
                }
                Some(b) if b == quote => {
                    let bytes = self.src[body_start..self.pos].to_vec();
                    self.advance();
                    return TokenKind::String(bytes);
                }
                _ => {
                    self.advance();
                }
            }
        }
    }
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_cont(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(src: &str) -> (Vec<TokenKind>, Vec<Diagnostic>) {
        let (toks, diags) = tokenize(src);
        (toks.into_iter().map(|t| t.kind).collect(), diags)
    }

    #[test]
    fn lexes_decimal() {
        let (k, d) = lex("42");
        assert_eq!(k, vec![TokenKind::Number(42), TokenKind::Eof]);
        assert!(d.is_empty());
    }

    #[test]
    fn lexes_hex_lowercase_and_uppercase() {
        let (k, _) = lex("0a000h 0FFh");
        assert_eq!(
            k,
            vec![
                TokenKind::Number(0xa000),
                TokenKind::Number(0xff),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_binary() {
        let (k, _) = lex("1010b 0B");
        assert_eq!(
            k,
            vec![TokenKind::Number(10), TokenKind::Number(0), TokenKind::Eof]
        );
    }

    #[test]
    fn ident_lowercased() {
        let (k, _) = lex("MOV Ax");
        assert_eq!(
            k,
            vec![
                TokenKind::Ident("mov".into()),
                TokenKind::Ident("ax".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn punctuation() {
        let (k, _) = lex("[],:+-*/%()");
        assert_eq!(
            k,
            vec![
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Comma,
                TokenKind::Colon,
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn string_and_char() {
        let (k, _) = lex("'A' 'hello$'");
        assert_eq!(
            k,
            vec![
                TokenKind::String(vec![b'A']),
                TokenKind::String(b"hello$".to_vec()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn comments_skipped() {
        let (k, _) = lex("mov ax, 1 ; this is a comment\nmov bx, 2");
        assert_eq!(
            k,
            vec![
                TokenKind::Ident("mov".into()),
                TokenKind::Ident("ax".into()),
                TokenKind::Comma,
                TokenKind::Number(1),
                TokenKind::Newline,
                TokenKind::Ident("mov".into()),
                TokenKind::Ident("bx".into()),
                TokenKind::Comma,
                TokenKind::Number(2),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn crlf_treated_as_single_newline() {
        let (k, _) = lex("a\r\nb");
        assert_eq!(
            k,
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::Newline,
                TokenKind::Ident("b".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn invalid_number_reports_diagnostic() {
        let (_, d) = lex("12xy");
        assert_eq!(d.len(), 1);
        assert!(d[0].message.contains("invalid number"));
    }

    #[test]
    fn unterminated_string_reports_diagnostic() {
        let (_, d) = lex("'oops");
        assert_eq!(d.len(), 1);
        assert!(d[0].message.contains("unterminated"));
    }

    #[test]
    fn span_tracks_line_col() {
        let (toks, _) = tokenize("ax\n  bx");
        assert_eq!(toks[0].span, Span::new(1, 1, 2));
        assert_eq!(toks[1].span, Span::new(1, 3, 1));
        assert_eq!(toks[2].span, Span::new(2, 3, 2));
    }
}
