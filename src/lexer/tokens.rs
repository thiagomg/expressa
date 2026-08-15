// 7. Suggested order of work (one PR/session each)
//
// 1. Span + Token + TokenKind (minimal) + tokenize returning Eof only
// 2. Whitespace + // comments
// 3. Numbers (with _ and .)
// 4. Idents + keyword table
// 5. Single-char ops, then multi-char
// 6. Strings + escapes
// 7. /* */
// 8. Print helper + tests from the spec examples (sections 2–8)

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
    De,         // de
    Ate,        // ate
    Em,         // em
    Repita,     // repita
    Vezes,      // vezes
    Mapa,       // mapa
    Importe,    // importe
    Contem,     // contem (infix operator, but reserved word)
    Verdadeiro, // verdadeiro
    Falso,      // falso
    SeFalhar,   // se_falhar   (ONE keyword, not Se + Ident)

    // ── Arithmetic ────────────────────────────────────────────────
    Plus,       // +
    Minus,      // -   (binary or unary — parser decides)
    Star,       // *
    Slash,      // /
    Percent,    // %

    // ── Comparison ────────────────────────────────────────────────
    EqEq,       // ==
    BangEq,     // !=
    Lt,         // <
    Gt,         // >
    LtEq,       // <=
    GtEq,       // >=

    // ── Assignment & map arrow ────────────────────────────────────
    Eq,         // =
    Arrow,      // ->

    // ── Range / access ────────────────────────────────────────────
    DotDot,     // ..   (only used inside [i..j])
    Dot,        // .    (module field: mat.soma)

    // ── Delimiters ────────────────────────────────────────────────
    LParen,     // (
    RParen,     // )
    LBracket,   // [
    RBracket,   // ]
    LBrace,     // {
    RBrace,     // }
    Comma,      // ,

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
        "de" => TokenKind::De,
        "ate" | "até" => TokenKind::Ate,
        "em" => TokenKind::Em,
        "repita" => TokenKind::Repita,
        "vezes" => TokenKind::Vezes,
        "mapa" => TokenKind::Mapa,
        "importe" => TokenKind::Importe,
        "contem" | "contém" => TokenKind::Contem,
        "verdadeiro" => TokenKind::Verdadeiro,
        "falso" => TokenKind::Falso,
        "se_falhar" => TokenKind::SeFalhar,
        _ => return None,
    })
}
