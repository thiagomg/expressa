use crate::lexer::Span;
use crate::lexer::scanner::{LexError, LexErrorKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
    Syntax,
    UnexpectedEof,
    Lexical(LexErrorKind),
    MissingElse,
    DepthLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub message: String,
    pub span: Span,
}

impl ParseError {
    pub fn is_incomplete(&self) -> bool {
        match self.kind {
            ParseErrorKind::UnexpectedEof => true,
            ParseErrorKind::Lexical(kind) => kind.is_incomplete(),
            _ => false,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} em {}", self.message, self.span)
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        Self {
            kind: ParseErrorKind::Lexical(e.kind),
            message: e.message,
            span: e.span,
        }
    }
}
