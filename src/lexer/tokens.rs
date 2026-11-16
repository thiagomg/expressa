#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // ── Literals & names ──────────────────────────────────────────
    /// Decimal number lexeme as written (e.g. "10", "3.14", "1_000").
    /// This is the raw text; Decimal will parse later in the runtime.
    Number(String),

    /// String contents already unescaped (e.g. hello\nworld → real newline).
    String(String),

    /// Non-keyword identifier.
    Ident(String),

    /// `///` documentation on the line immediately above a definition.
    /// `////` is a normal `//` comment, not this.
    DocComment(String),

    // ── Keywords (Portuguese) ─────────────────────────────────────
    Se,         // se
    Senao,      // senao
    Ou,         // ou (logical OR *and* part of "ou se")
    E,          // e
    Nao,        // nao
    Inicio,     // inicio
    Fim,        // fim
    Funcao,     // funcao
    Para,       // para
    Em,         // em
    Repita,     // repita
    Enquanto,   // enquanto
    Vezes,      // vezes
    Importe,    // importe
    Contem,     // contem (infix operator, but reserved word)
    Verdadeiro, // verdadeiro
    Falso,      // falso
    SeFalhar,   // se_falhar
    Retorne,    // retorne
    Pare,       // pare
    Continue,   // continue / continua   (ONE keyword, not Se + Ident)

    // ── Arithmetic ────────────────────────────────────────────────
    Plus,    // +
    Minus,   // -   (binary or unary — parser decides)
    Star,    // *
    Slash,   // /
    Percent, // %

    // ── Comparison ────────────────────────────────────────────────
    EqEq,   // ==
    BangEq, // !=
    Lt,     // <
    Gt,     // >
    LtEq,   // <=
    GtEq,   // >=

    // ── Assignment & map arrow ────────────────────────────────────
    Eq,      // =
    PlusEq,  // +=
    MinusEq, // -=
    Arrow,   // ->

    // ── Range / access ────────────────────────────────────────────
    DotDot,     // ..
    DotDotEq,   // ..=
    Dot,        // .    (UFCS: xs.tamanho())
    Colon,      // :    (mapa key: pessoa:nome)
    ColonColon, // ::   (módulo: mat::soma)

    // ── Delimiters ────────────────────────────────────────────────
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    LBrace,   // {
    RBrace,   // }
    Comma,    // ,

    // ── End ───────────────────────────────────────────────────────
    Eof,
}

pub fn keyword(s: &str) -> Option<TokenKind> {
    Some(match s {
        "se" => TokenKind::Se,
        "senao" | "senão" => TokenKind::Senao,
        "ou" => TokenKind::Ou,
        "e" => TokenKind::E,
        "nao" | "não" => TokenKind::Nao,
        "inicio" | "início" => TokenKind::Inicio,
        "fim" => TokenKind::Fim,
        "funcao" | "função" => TokenKind::Funcao,
        "para" => TokenKind::Para,
        "em" => TokenKind::Em,
        "repita" => TokenKind::Repita,
        "enquanto" => TokenKind::Enquanto,
        "vezes" => TokenKind::Vezes,
        "importe" => TokenKind::Importe,
        "contem" | "contém" => TokenKind::Contem,
        "verdadeiro" => TokenKind::Verdadeiro,
        "falso" => TokenKind::Falso,
        "se_falhar" => TokenKind::SeFalhar,
        "retorne" => TokenKind::Retorne,
        "pare" => TokenKind::Pare,
        "continue" | "continua" => TokenKind::Continue,
        _ => return None,
    })
}

/// Source spelling for diagnostics. Symbols keep their quotes (`'+='`);
/// `Eof` is the Portuguese phrase `fim do arquivo`.
pub fn texto_do_token(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Number(n) => format!("número {n}"),
        TokenKind::String(s) => format!("\"{s}\""),
        TokenKind::Ident(n) => format!("'{n}'"),
        TokenKind::DocComment(_) => "documentação".to_string(),
        TokenKind::Se => "'se'".to_string(),
        TokenKind::Senao => "'senao'".to_string(),
        TokenKind::Ou => "'ou'".to_string(),
        TokenKind::E => "'e'".to_string(),
        TokenKind::Nao => "'nao'".to_string(),
        TokenKind::Inicio => "'inicio'".to_string(),
        TokenKind::Fim => "'fim'".to_string(),
        TokenKind::Funcao => "'funcao'".to_string(),
        TokenKind::Para => "'para'".to_string(),
        TokenKind::Em => "'em'".to_string(),
        TokenKind::Repita => "'repita'".to_string(),
        TokenKind::Enquanto => "'enquanto'".to_string(),
        TokenKind::Vezes => "'vezes'".to_string(),
        TokenKind::Importe => "'importe'".to_string(),
        TokenKind::Contem => "'contem'".to_string(),
        TokenKind::Verdadeiro => "'verdadeiro'".to_string(),
        TokenKind::Falso => "'falso'".to_string(),
        TokenKind::SeFalhar => "'se_falhar'".to_string(),
        TokenKind::Retorne => "'retorne'".to_string(),
        TokenKind::Pare => "'pare'".to_string(),
        TokenKind::Continue => "'continue'".to_string(),
        TokenKind::Plus => "'+'".to_string(),
        TokenKind::Minus => "'-'".to_string(),
        TokenKind::Star => "'*'".to_string(),
        TokenKind::Slash => "'/'".to_string(),
        TokenKind::Percent => "'%'".to_string(),
        TokenKind::EqEq => "'=='".to_string(),
        TokenKind::BangEq => "'!='".to_string(),
        TokenKind::Lt => "'<'".to_string(),
        TokenKind::Gt => "'>'".to_string(),
        TokenKind::LtEq => "'<='".to_string(),
        TokenKind::GtEq => "'>='".to_string(),
        TokenKind::Eq => "'='".to_string(),
        TokenKind::PlusEq => "'+='".to_string(),
        TokenKind::MinusEq => "'-='".to_string(),
        TokenKind::Arrow => "'->'".to_string(),
        TokenKind::DotDot => "'..'".to_string(),
        TokenKind::DotDotEq => "'..='".to_string(),
        TokenKind::Dot => "'.'".to_string(),
        TokenKind::Colon => "':'".to_string(),
        TokenKind::ColonColon => "'::'".to_string(),
        TokenKind::LParen => "'('".to_string(),
        TokenKind::RParen => "')'".to_string(),
        TokenKind::LBracket => "'['".to_string(),
        TokenKind::RBracket => "']'".to_string(),
        TokenKind::LBrace => "'{'".to_string(),
        TokenKind::RBrace => "'}'".to_string(),
        TokenKind::Comma => "','".to_string(),
        TokenKind::Eof => "fim do arquivo".to_string(),
    }
}

/// Keywords that only start a command. `se` and `funcao` are also expressions.
pub fn token_e_comando(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Pare
            | TokenKind::Continue
            | TokenKind::Retorne
            | TokenKind::Para
            | TokenKind::Enquanto
            | TokenKind::Repita
            | TokenKind::Importe
            | TokenKind::Senao
            | TokenKind::Fim
            | TokenKind::Vezes
            | TokenKind::Em
    )
}
