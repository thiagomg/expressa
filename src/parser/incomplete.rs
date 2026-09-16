use crate::lexer::TokenKind;
use crate::lexer::scanner::tokenize;
use crate::parser::parse;

/// Whether the snippet still needs more lines (open `inicio`, `(`, string, …).
pub fn needs_more_input(source: &str) -> bool {
    if source.trim().is_empty() {
        return false;
    }
    match tokenize(source) {
        Err(e) => {
            e.message.contains("não fechada")
                || e.message.contains("não fechado")
                || e.message.contains("incompleto")
                || e.message.contains("fim do arquivo")
        }
        Ok(tokens) => {
            if !delimiters_ok(&tokens) {
                return true;
            }
            match parse(source) {
                Ok(_) => false,
                Err(e) => {
                    let eof_start = tokens.last().map(|t| t.span.start).unwrap_or(0);
                    e.span.start >= eof_start
                }
            }
        }
    }
}

fn delimiters_ok(tokens: &[crate::lexer::Token]) -> bool {
    let mut inicio = 0i32;
    let mut paren = 0i32;
    let mut brack = 0i32;
    let mut brace = 0i32;
    for t in tokens {
        match t.kind {
            TokenKind::Inicio => inicio += 1,
            TokenKind::Fim => inicio -= 1,
            TokenKind::LParen => paren += 1,
            TokenKind::RParen => paren -= 1,
            TokenKind::LBracket => brack += 1,
            TokenKind::RBracket => brack -= 1,
            TokenKind::LBrace => brace += 1,
            TokenKind::RBrace => brace -= 1,
            _ => {}
        }
        if inicio < 0 || paren < 0 || brack < 0 || brace < 0 {
            return true;
        }
    }
    inicio == 0 && paren == 0 && brack == 0 && brace == 0
}

#[cfg(test)]
mod tests {
    use super::needs_more_input;

    #[test]
    fn complete_snippets() {
        assert!(!needs_more_input("2 + 2"));
        assert!(!needs_more_input("x = 1"));
        assert!(!needs_more_input("escreva(1)"));
        assert!(!needs_more_input("2 + * 3"));
    }

    #[test]
    fn incomplete_snippets() {
        assert!(needs_more_input("2 +"));
        assert!(needs_more_input("inicio"));
        assert!(needs_more_input("se verdadeiro"));
        assert!(needs_more_input("soma = funcao(a, b)"));
        assert!(needs_more_input("escreva("));
        assert!(needs_more_input("\"abc"));
        assert!(needs_more_input("lista = [1,"));
        assert!(needs_more_input("se verdadeiro {"));
        assert!(needs_more_input("funcao(n) {"));
        assert!(!needs_more_input("funcao(n) { n }"));
    }
}
