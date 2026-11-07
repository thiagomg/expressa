//! Tree-sitter grammar for Expressa.

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_expressa() -> *const ();
}

/// Language function consumed by `freya-code-editor`.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_expressa) };

pub const HIGHLIGHTS_QUERY: &str = include_str!("../../queries/highlights.scm");

#[cfg(test)]
mod tests {
    #[test]
    fn carrega_a_gramatica() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("gramática Expressa");
    }

    #[test]
    fn destaca_palavras_reservadas() {
        assert!(super::HIGHLIGHTS_QUERY.contains("@keyword"));
        assert!(super::HIGHLIGHTS_QUERY.contains("@string"));
        assert!(super::HIGHLIGHTS_QUERY.contains("@comment"));
    }

    #[test]
    fn analisa_funcao_com_acento() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("gramática Expressa");
        let tree = parser.parse("função x\n\"início\"\n", None).expect("árvore");
        let root = tree.root_node();
        assert!(!root.has_error());
        let dump = root.to_sexp();
        assert!(dump.contains("keyword"), "{dump}");
        assert!(dump.contains("string"), "{dump}");
    }
}
