use crate::lexer::{Span, TokenKind};
use crate::lexer::tokens::keyword;

struct Scanner<'src> {
    source: &'src str,
    pos: usize,
    line: u32,
    col: u32,
}

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
    let mut scanner = Scanner::new(source);
    let mut tokens = Vec::new();
    loop {
        let tok = scanner.next_token()?;
        let is_eof = tok.kind == TokenKind::Eof;
        tokens.push(tok);
        if is_eof {
            break;
        }
    }
    Ok(tokens)
}

impl<'src> Scanner<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            pos: 0,
            line: 1,
            col: 1
        }
    }

    pub fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    pub fn skip_peek(&self, skip_count: usize) -> Option<char> {
        self.source[self.pos..].chars().skip(skip_count).next()
    }

    pub fn remove(&mut self) -> Option<char> {
        let c = self.peek();
        if c == None {
            return None;
        }
        let c = c.unwrap();
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        self.pos += c.len_utf8();

        Some(c)
    }

    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_non_code()?;

        let line = self.line;
        let col = self.col;
        let start = self.pos;

        let ch = match self.peek() {
            None => return Ok(Token {
                kind: TokenKind::Eof,
                span: Span {
                    line,
                    col,
                    start,
                    end: self.pos,
                },
            }),
            Some(c) => c,
        };

        macro_rules! token {
            ($kind:expr) => {
                Ok(Token {
                    kind: $kind,
                    span: Span {
                        line,
                        col,
                        start,
                        end: self.pos,
                    }
                })
            };
        }

        macro_rules! lex_err {
            ($($arg:tt)*) => {
                Err(LexError {
                    message: format!($($arg)*),
                    span: Span {
                        line,
                        col,
                        start,
                        end: self.pos,
                    }
                })
            };
        }

        // Ident / keyword
        if Self::is_ident_start(ch) {
            while let Some(c) = self.peek() {
                if Self::is_ident_continue(c) {
                    self.remove();
                } else {
                    break;
                }
            }
            let word = &self.source[start..self.pos];
            let kind = keyword(word).unwrap_or_else(|| TokenKind::Ident(word.to_string()));
            return token!(kind);
        }

        // Number (does not include leading '-')
        // Do not consume `..` (range) as part of a number.
        if ch.is_ascii_digit() {
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() || c == '_' {
                    self.remove();
                } else {
                    break;
                }
            }
            // Optional fractional part: '.' only if followed by a digit
            if self.peek() == Some('.') && self.skip_peek(1).is_some_and(|c| c.is_ascii_digit()) {
                self.remove(); // .
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() || c == '_' {
                        self.remove();
                    } else {
                        break;
                    }
                }
            }
            let word = &self.source[start..self.pos];
            let kind = TokenKind::Number(word.to_string());
            return token!(kind);
        }

        // String
        if ch == '"' {
            let value = self.scan_string(line, col, start)?;
            return token!(TokenKind::String(value));
        }

        match ch {
            '=' => {
                self.remove();
                if self.peek() == Some('=') {
                    self.remove();
                    token!(TokenKind::EqEq)
                } else {
                    token!(TokenKind::Eq)
                }
            }
            '!' => {
                self.remove();
                if self.peek() == Some('=') {
                    self.remove();
                    token!(TokenKind::BangEq)
                } else {
                    lex_err!("caractere '!' inválido; você quis '!='?")
                }
            }
            '<' => {
                self.remove();
                if self.peek() == Some('=') {
                    self.remove();
                    token!(TokenKind::LtEq)
                } else {
                    token!(TokenKind::Lt)
                }
            }
            '>' => {
                self.remove();
                if self.peek() == Some('=') {
                    self.remove();
                    token!(TokenKind::GtEq)
                } else {
                    token!(TokenKind::Gt)
                }
            }
            '-' => {
                self.remove();
                if self.peek() == Some('>') {
                    self.remove();
                    token!(TokenKind::Arrow)
                } else {
                    token!(TokenKind::Minus)
                }
            }
            '.' => {
                self.remove();
                if self.peek() == Some('.') {
                    self.remove();
                    token!(TokenKind::DotDot)
                } else {
                    token!(TokenKind::Dot)
                }
            }
            '/' => {
                // comments handled in skip_trivia; bare '/' is division
                self.remove();
                token!(TokenKind::Slash)
            }
            '+' => {
                self.remove();
                token!(TokenKind::Plus)
            }
            '*' => {
                self.remove();
                token!(TokenKind::Star)
            }
            '%' => {
                self.remove();
                token!(TokenKind::Percent)
            }
            '(' => {
                self.remove();
                token!(TokenKind::LParen)
            }
            ')' => {
                self.remove();
                token!(TokenKind::RParen)
            }
            '[' => {
                self.remove();
                token!(TokenKind::LBracket)
            }
            ']' => {
                self.remove();
                token!(TokenKind::RBracket)
            }
            '{' => {
                self.remove();
                token!(TokenKind::LBrace)
            }
            '}' => {
                self.remove();
                token!(TokenKind::RBrace)
            }
            ',' => {
                self.remove();
                token!(TokenKind::Comma)
            }
            _ => {
                self.remove();
                lex_err!("caractere inválido: {ch:?}")
            }
        }
    }

    fn skip_non_code(&mut self) -> Result<(), LexError> {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.remove();
                }
                Some('/') if self.skip_peek(1) == Some('/') => {
                    self.remove(); // /
                    self.remove(); // /
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.remove();
                    }
                }
                Some('/') if self.skip_peek(1) == Some('*') => {
                    let line = self.line;
                    let col = self.col;
                    let start = self.pos;
                    self.remove(); // /
                    self.remove(); // *
                    loop {
                        match self.peek() {
                            None => {
                                return Err(LexError {
                                    message: "comentário de bloco não fechado".to_string(),
                                    span: Span {
                                        line,
                                        col,
                                        start,
                                        end: self.pos,
                                    }
                                });
                            }
                            Some('*') if self.skip_peek(1) == Some('/') => {
                                self.remove();
                                self.remove();
                                break;
                            }
                            Some(_) => {
                                self.remove();
                            }
                        }
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    /// Scan a string literal starting at the current `"`.
    ///
    /// Supports escapes: `\\` `\"` `\n` `\r` `\t` `\u{...}`.
    /// Raw newlines inside the string are allowed (spec).
    /// Returns the unescaped contents (without surrounding quotes).
    fn scan_string(
        &mut self,
        line: u32,
        col: u32,
        start: usize,
    ) -> Result<String, LexError> {
        self.remove(); // opening "

        let mut value = String::new();

        loop {
            match self.peek() {
                None => {
                    return Err(LexError {
                        message: "string não fechada".to_string(),
                        span: Span {
                            line,
                            col,
                            start,
                            end: self.pos,
                        },
                    });
                }
                Some('"') => {
                    self.remove(); // closing "
                    break;
                }
                Some('\\') => {
                    self.remove(); // backslash
                    let esc_line = self.line;
                    let esc_col = self.col;
                    let esc_start = self.pos;

                    let Some(esc) = self.remove() else {
                        return Err(LexError {
                            message: "escape incompleto no fim do arquivo".to_string(),
                            span: Span {
                                line,
                                col,
                                start,
                                end: self.pos,
                            },
                        });
                    };

                    match esc {
                        '\\' => value.push('\\'),
                        '"' => value.push('"'),
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        'u' => {
                            value.push(self.scan_unicode_escape(esc_line, esc_col, esc_start)?);
                        }
                        other => {
                            return Err(LexError {
                                message: format!("escape inválido: \\{other}"),
                                span: Span {
                                    line: esc_line,
                                    col: esc_col,
                                    start: esc_start,
                                    end: self.pos,
                                },
                            });
                        }
                    }
                }
                Some(c) => {
                    // Includes raw newlines (multiline strings).
                    self.remove();
                    value.push(c);
                }
            }
        }

        Ok(value)
    }

    /// Parse `\u{HEX}` after the `u` has already been consumed.
    fn scan_unicode_escape(
        &mut self,
        esc_line: u32,
        esc_col: u32,
        esc_start: usize,
    ) -> Result<char, LexError> {
        if self.peek() != Some('{') {
            return Err(LexError {
                message: "escape \\u deve ser \\u{...}".to_string(),
                span: Span {
                    line: esc_line,
                    col: esc_col,
                    start: esc_start,
                    end: self.pos,
                },
            });
        }
        self.remove(); // {

        let hex_start = self.pos;
        while let Some(c) = self.peek() {
            if c == '}' {
                break;
            }
            if c.is_ascii_hexdigit() {
                self.remove();
            } else {
                return Err(LexError {
                    message: "dígito hexadecimal inválido em \\u{...}".to_string(),
                    span: Span {
                        line: self.line,
                        col: self.col,
                        start: self.pos,
                        end: self.pos + c.len_utf8(),
                    },
                });
            }
        }

        if self.peek() != Some('}') {
            return Err(LexError {
                message: "\\u{...} não fechado".to_string(),
                span: Span {
                    line: esc_line,
                    col: esc_col,
                    start: esc_start,
                    end: self.pos,
                },
            });
        }

        let hex = &self.source[hex_start..self.pos];
        if hex.is_empty() {
            self.remove(); // }
            return Err(LexError {
                message: "\\u{} vazio".to_string(),
                span: Span {
                    line: esc_line,
                    col: esc_col,
                    start: esc_start,
                    end: self.pos,
                },
            });
        }

        self.remove(); // }

        let code = u32::from_str_radix(hex, 16).map_err(|_| LexError {
            message: "código Unicode inválido".to_string(),
            span: Span {
                line: esc_line,
                col: esc_col,
                start: esc_start,
                end: self.pos,
            },
        })?;

        char::from_u32(code).ok_or_else(|| LexError {
            message: "código Unicode inválido".to_string(),
            span: Span {
                line: esc_line,
                col: esc_col,
                start: esc_start,
                end: self.pos,
            },
        })
    }

    fn is_ident_start(c: char) -> bool {
        c.is_alphabetic() || c == '_'
    }

    fn is_ident_continue(c: char) -> bool {
        c.is_alphabetic() || c.is_ascii_digit() || c == '_'
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_peek() {
        let src = "Teste do Thiago";
        let s = Scanner::new(src);
        assert_eq!(s.peek(), Some('T'));
        assert_eq!(s.peek(), Some('T'));

        let src = "ão";
        let s = Scanner::new(src);
        assert_eq!(s.peek(), Some('ã'));
        assert_eq!(s.peek(), Some('ã'));
    }

    #[test]
    pub fn test_skip_peek() {
        let src = "Tão grande";
        assert_eq!(src.len(), 11);
        assert_eq!(src.chars().count(), 10);
        let s = Scanner::new(src);
        assert_eq!(s.skip_peek(0), Some('T'));
        assert_eq!(s.skip_peek(1), Some('ã'));
        assert_eq!(s.skip_peek(2), Some('o'));
        assert_eq!(s.skip_peek(9), Some('e'));
    }

    #[test]
    pub fn test_remove() {
        let src = "Tão";
        assert_eq!(src.len(), 4);
        assert_eq!(src.chars().count(), 3);
        let mut s = Scanner::new(src);
        assert_eq!(s.remove(), Some('T'));
        assert_eq!(s.remove(), Some('ã'));
        assert_eq!(s.remove(), Some('o'));
        assert_eq!(s.remove(), None);
        assert_eq!(src.chars().count(), 3);
    }

    #[test]
    pub fn test_next_tok() {
        let src = "===";
        let mut s = Scanner::new(src);
        let tok = s.next_token().unwrap();
        println!("{:?}", tok);
        let tok = s.next_token().unwrap();
        println!("{:?}", tok);
        let tok = s.next_token().unwrap();
        println!("{:?}", tok);
    }

    #[test]
    pub fn test_next_tok_err() {
        let src = "@";
        let mut s = Scanner::new(src);
        let tok = s.next_token();
        println!("{:?}", tok);
    }

    /// Kinds only (ignore spans) so the assert stays readable.
    fn kinds(source: &str) -> Vec<TokenKind> {
        tokenize(source)
            .expect("tokenize should succeed")
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn operators_assignment_comparison_arrow_range() {
        // Space-separated so each operator is its own token.
        let src = r#"= == != < > <= >= -> .. + - * / %"#;

        assert_eq!(
            kinds(src),
            vec![
                TokenKind::Eq,      // =
                TokenKind::EqEq,    // ==
                TokenKind::BangEq,  // !=
                TokenKind::Lt,      // <
                TokenKind::Gt,      // >
                TokenKind::LtEq,    // <=
                TokenKind::GtEq,    // >=
                TokenKind::Arrow,   // ->
                TokenKind::DotDot,  // ..
                TokenKind::Plus,    // +
                TokenKind::Minus,   // -
                TokenKind::Star,    // *
                TokenKind::Slash,   // /
                TokenKind::Percent, // %
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn bang_alone_is_error() {
        let err = tokenize("!").expect_err("bare ! is not a token");
        let _ = err;
    }

    #[test]
    fn operators_without_spaces_1() {
        let src = "a==b=c>=d>w<=f<g!=h->i..j.k";

        let x = kinds(src);
        println!("{:?}", x);
    }

    #[test]
    fn operators_without_spaces_do_not_merge_wrongly() {
        // Classic pitfalls: == vs =, >= vs >, -> vs -, .. vs .
        // Avoid single-letter keywords (e, de, em, se, ou, …) as fake idents.
        let src = "a==b=c>=d>w<=f<g!=h->i..j.k";

        assert_eq!(
            kinds(src),
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::EqEq,
                TokenKind::Ident("b".into()),
                TokenKind::Eq,
                TokenKind::Ident("c".into()),
                TokenKind::GtEq,
                TokenKind::Ident("d".into()),
                TokenKind::Gt,
                TokenKind::Ident("w".into()),
                TokenKind::LtEq,
                TokenKind::Ident("f".into()),
                TokenKind::Lt,
                TokenKind::Ident("g".into()),
                TokenKind::BangEq,
                TokenKind::Ident("h".into()),
                TokenKind::Arrow,
                TokenKind::Ident("i".into()),
                TokenKind::DotDot,
                TokenKind::Ident("j".into()),
                TokenKind::Dot,
                TokenKind::Ident("k".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn double_equals_is_not_two_eq_tokens() {
        // Uses idents on the RHS until number() is implemented.
        let ks = kinds("x == y");
        assert_eq!(
            ks,
            vec![
                TokenKind::Ident("x".into()),
                TokenKind::EqEq, // one token, not Eq Eq
                TokenKind::Ident("y".into()),
                TokenKind::Eof,
            ]
        );

        // Single = must stay assignment
        assert_eq!(
            kinds("x = y"),
            vec![
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::Ident("y".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn identifiers_are_not_keywords() {
        assert_eq!(
            kinds("x soma _priv lista2 média"),
            vec![
                TokenKind::Ident("x".into()),
                TokenKind::Ident("soma".into()),
                TokenKind::Ident("_priv".into()),
                TokenKind::Ident("lista2".into()),
                TokenKind::Ident("média".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn keyword_prefix_of_ident_stays_ident() {
        // Whole-word match only: "senao_algo" is not Senao.
        assert_eq!(
            kinds("senao_algo se_falhar_x senãox função2"),
            vec![
                TokenKind::Ident("senao_algo".into()),
                TokenKind::Ident("se_falhar_x".into()),
                TokenKind::Ident("senãox".into()),
                TokenKind::Ident("função2".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn all_reserved_words_ascii() {
        // One source with every ASCII keyword from the language (space-separated).
        let src = "\
se senao ou e nao inicio fim funcao para de ate em \
repita vezes mapa importe contem verdadeiro falso se_falhar";

        assert_eq!(
            kinds(src),
            vec![
                TokenKind::Se,
                TokenKind::Senao,
                TokenKind::Ou,
                TokenKind::E,
                TokenKind::Nao,
                TokenKind::Inicio,
                TokenKind::Fim,
                TokenKind::Funcao,
                TokenKind::Para,
                TokenKind::De,
                TokenKind::Ate,
                TokenKind::Em,
                TokenKind::Repita,
                TokenKind::Vezes,
                TokenKind::Mapa,
                TokenKind::Importe,
                TokenKind::Contem,
                TokenKind::Verdadeiro,
                TokenKind::Falso,
                TokenKind::SeFalhar,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn reserved_words_with_accents() {
        // Accented spellings accepted by keyword().
        let src = "senão não início função até contém";

        assert_eq!(
            kinds(src),
            vec![
                TokenKind::Senao,
                TokenKind::Nao,
                TokenKind::Inicio,
                TokenKind::Funcao,
                TokenKind::Ate,
                TokenKind::Contem,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn ou_se_are_two_tokens() {
        assert_eq!(
            kinds("ou se"),
            vec![TokenKind::Ou, TokenKind::Se, TokenKind::Eof]
        );
    }

    #[test]
    fn se_falhar_is_one_keyword() {
        assert_eq!(
            kinds("se_falhar"),
            vec![TokenKind::SeFalhar, TokenKind::Eof]
        );
        // Not Se + Ident("falhar")
        assert_ne!(
            kinds("se_falhar"),
            vec![
                TokenKind::Se,
                TokenKind::Ident("falhar".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn keywords_and_idents_mixed() {
        // No number literals here (covered when number() is implemented).
        let src = "se x e y senao z se_falhar w";

        assert_eq!(
            kinds(src),
            vec![
                TokenKind::Se,
                TokenKind::Ident("x".into()),
                TokenKind::E,
                TokenKind::Ident("y".into()),
                TokenKind::Senao,
                TokenKind::Ident("z".into()),
                TokenKind::SeFalhar,
                TokenKind::Ident("w".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn string_simple() {
        assert_eq!(
            kinds(r#""olá""#),
            vec![TokenKind::String("olá".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r#""""#),
            vec![TokenKind::String("".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn string_escapes() {
        assert_eq!(
            kinds(r#""a\nb\tc\\d\"e""#),
            vec![
                TokenKind::String("a\nb\tc\\d\"e".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn string_unicode_escape() {
        // 😀 = U+1F600
        assert_eq!(
            kinds(r#""oi\u{1F600}""#),
            vec![TokenKind::String("oi😀".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r#""\u{41}""#),
            vec![TokenKind::String("A".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn string_multiline() {
        let src = "\"linha1\nlinha2\"";
        assert_eq!(
            kinds(src),
            vec![
                TokenKind::String("linha1\nlinha2".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn string_in_assignment() {
        assert_eq!(
            kinds(r#"nome = "Ana""#),
            vec![
                TokenKind::Ident("nome".into()),
                TokenKind::Eq,
                TokenKind::String("Ana".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn string_unclosed_is_error() {
        let err = tokenize(r#""sem fechar"#).expect_err("unclosed string");
        assert!(err.message.contains("não fechada"));
    }

    #[test]
    fn string_invalid_escape_is_error() {
        let err = tokenize(r#""\q""#).expect_err("bad escape");
        assert!(err.message.contains("escape inválido"));
    }

    #[test]
    fn string_bad_unicode_escape_is_error() {
        assert!(tokenize(r#""\u41""#).is_err());
        assert!(tokenize(r#""\u{}"#).is_err());
        assert!(tokenize(r#""\u{110000}"#).is_err()); // beyond Unicode scalar
        assert!(tokenize(r#""\u{zz}"#).is_err());
    }

    #[test]
    fn string_cr_escape() {
        assert_eq!(
            kinds(r#""a\rb""#),
            vec![TokenKind::String("a\rb".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn string_adjacent_to_punct() {
        assert_eq!(
            kinds(r#"escreva("oi")"#),
            vec![
                TokenKind::Ident("escreva".into()),
                TokenKind::LParen,
                TokenKind::String("oi".into()),
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn string_escape_at_eof_is_error() {
        let err = tokenize("\"\\").expect_err("backslash at eof");
        assert!(
            err.message.contains("escape") || err.message.contains("não fechada"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn string_span_covers_quotes() {
        let src = r#"  "hi""#;
        let tokens = tokenize(src).unwrap();
        let t = &tokens[0];
        assert_eq!(t.kind, TokenKind::String("hi".into()));
        assert_eq!(t.span.line, 1);
        assert_eq!(t.span.col, 3);
        assert_eq!(&src[t.span.start..t.span.end], r#""hi""#);
    }

    #[test]
    fn numbers_integers_decimals_underscores() {
        assert_eq!(
            kinds("0 42 3.14 1_000 10.5"),
            vec![
                TokenKind::Number("0".into()),
                TokenKind::Number("42".into()),
                TokenKind::Number("3.14".into()),
                TokenKind::Number("1_000".into()),
                TokenKind::Number("10.5".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn number_negative_is_minus_plus_number() {
        assert_eq!(
            kinds("-8"),
            vec![
                TokenKind::Minus,
                TokenKind::Number("8".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn line_comment_is_skipped() {
        assert_eq!(
            kinds("x // comentário\ny"),
            vec![
                TokenKind::Ident("x".into()),
                TokenKind::Ident("y".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn block_comment_is_skipped() {
        assert_eq!(
            kinds("a /* multi\nlinha */ b"),
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::Ident("b".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn unclosed_block_comment_is_error() {
        let err = tokenize("a /* sem fechar").expect_err("unclosed block comment");
        assert!(err.message.contains("comentário"));
    }

    #[test]
    fn delimiters() {
        assert_eq!(
            kinds("( ) [ ] { } ,"),
            vec![
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::Comma,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn mapa_vazio_tokens() {
        assert_eq!(
            kinds("mapa {}"),
            vec![
                TokenKind::Mapa,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn empty_source_is_only_eof() {
        assert_eq!(kinds(""), vec![TokenKind::Eof]);
        assert_eq!(kinds("   \n\t  "), vec![TokenKind::Eof]);
    }

    #[test]
    fn only_comments_is_only_eof() {
        assert_eq!(kinds("// só comentário"), vec![TokenKind::Eof]);
        assert_eq!(kinds("/* bloco */"), vec![TokenKind::Eof]);
    }

    #[test]
    fn realistic_snippet() {
        let src = r#"
// média
notas = [7.5, 8.0]
nome = "Ana"
se media >= 7
inicio
    escreva("ok") se_falhar falso
fim
"#;
        assert_eq!(
            kinds(src),
            vec![
                TokenKind::Ident("notas".into()),
                TokenKind::Eq,
                TokenKind::LBracket,
                TokenKind::Number("7.5".into()),
                TokenKind::Comma,
                TokenKind::Number("8.0".into()),
                TokenKind::RBracket,
                TokenKind::Ident("nome".into()),
                TokenKind::Eq,
                TokenKind::String("Ana".into()),
                TokenKind::Se,
                TokenKind::Ident("media".into()),
                TokenKind::GtEq,
                TokenKind::Number("7".into()),
                TokenKind::Inicio,
                TokenKind::Ident("escreva".into()),
                TokenKind::LParen,
                TokenKind::String("ok".into()),
                TokenKind::RParen,
                TokenKind::SeFalhar,
                TokenKind::Falso,
                TokenKind::Fim,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn invalid_char_is_error() {
        let err = tokenize("@").expect_err("invalid char");
        assert!(err.message.contains("inválido"));
        assert_eq!(err.span.line, 1);
        assert_eq!(err.span.col, 1);
    }
}
