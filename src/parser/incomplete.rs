use crate::parser::parse;

/// Whether appending input may finish an otherwise valid snippet.
/// Classification is independent of diagnostic wording and parses only once.
pub fn needs_more_input(source: &str) -> bool {
    parse(source).is_err_and(|error| error.is_incomplete())
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
        assert!(needs_more_input("enquanto verdadeiro"));
        assert!(needs_more_input("f = funcao() { retorne"));
        assert!(needs_more_input("funcao(n) {"));
        assert!(!needs_more_input("funcao(n) { n }"));
    }
}
