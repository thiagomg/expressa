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

    #[test]
    fn dois_pontos_ident_nao_e_variavel() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("gramática Expressa");
        let tree = parser
            .parse("g.cart::plote(função(x) { x }, :azul)\n", None)
            .expect("árvore");
        let dump = tree.root_node().to_sexp();
        assert!(dump.contains("key_ident"), "{dump}");
        assert!(
            !dump.contains("(identifier :azul)"),
            "{dump}"
        );
        assert!(super::HIGHLIGHTS_QUERY.contains("(key_ident) @string"));
    }

    #[test]
    fn gramatica_lista_as_keywords_da_aula() {
        let grammar = include_str!("../../grammar.js");
        for k in [
            "função",
            "funcao",
            "início",
            "inicio",
            "não",
            "nao",
            "senão",
            "senao",
        ] {
            assert!(grammar.contains(&format!("\"{k}\"")), "{k}");
        }
    }
}
