//! Immutable source identity and statement locations for one execution.
use expressa::parser::{AssignTarget, Block, Expr, Item, Stmt};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

#[derive(Debug)]
pub struct Source {
    pub path: String,
    pub text: String,
    pub revision: String,
    pub lines: Option<BTreeSet<u32>>,
}
impl Source {
    pub fn new(path: &str, text: String) -> Self {
        let lines = executable_lines(&text);
        Self {
            path: normalize_path(path),
            revision: expressa_aula_storage::revision(text.as_bytes()),
            text,
            lines,
        }
    }
    pub fn differs(&self, text: &str) -> bool {
        self.text != text
    }
}

pub type Sources = HashMap<String, Arc<Source>>;

pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

/// The evaluator calls `before_stmt` for statements, including statements in
/// function bodies and expression blocks; imports themselves have no hook.
pub fn executable_lines(text: &str) -> Option<BTreeSet<u32>> {
    let program = expressa::parser::parse(text).ok()?;
    let mut lines = BTreeSet::new();
    for item in &program.items {
        if let Item::Stmt(s) = item {
            stmt(s, &mut lines);
        }
    }
    Some(lines)
}
fn block(b: &Block, lines: &mut BTreeSet<u32>) {
    for s in &b.stmts {
        stmt(s, lines);
    }
}
fn stmt(s: &Stmt, lines: &mut BTreeSet<u32>) {
    lines.insert(s.span().line);
    match s {
        Stmt::Expr { expr: e, .. } => expr(e, lines),
        Stmt::Assign { target, value, .. } => {
            match target {
                AssignTarget::Name { .. } => (),
                AssignTarget::Index { object, index, .. } => {
                    expr(object, lines);
                    expr(index, lines);
                }
                AssignTarget::Index2 {
                    object, row, col, ..
                } => {
                    expr(object, lines);
                    expr(row, lines);
                    expr(col, lines);
                }
                AssignTarget::MapField { object, .. } => expr(object, lines),
            }
            expr(value, lines);
        }
        Stmt::Repita { count, body, .. } => {
            expr(count, lines);
            block(body, lines);
        }
        Stmt::ParaIn { iter, body, .. } => {
            expr(iter, lines);
            block(body, lines);
        }
        Stmt::Enquanto { cond, body, .. } => {
            expr(cond, lines);
            block(body, lines);
        }
        Stmt::Retorne { value, .. } => {
            if let Some(e) = value {
                expr(e, lines);
            }
        }
        Stmt::Pare { .. } | Stmt::Continue { .. } => (),
    }
}
fn expr(e: &Expr, lines: &mut BTreeSet<u32>) {
    match e {
        Expr::Function { body, .. } => block(body, lines),
        Expr::Block(body) => block(body, lines),
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            for b in branches {
                expr(&b.cond, lines);
                block(&b.body, lines);
            }
            if let Some(b) = else_block {
                block(b, lines);
            }
        }
        Expr::List { elements, .. } => {
            for e in elements {
                expr(e, lines);
            }
        }
        Expr::Call { callee, args, .. } => {
            expr(callee, lines);
            for e in args {
                expr(e, lines);
            }
        }
        Expr::Par { key, value, .. } => {
            expr(key, lines);
            expr(value, lines);
        }
        Expr::Binary { left, right, .. } => {
            expr(left, lines);
            expr(right, lines);
        }
        Expr::SeFalhar {
            attempt, fallback, ..
        } => {
            expr(attempt, lines);
            expr(fallback, lines);
        }
        Expr::Unary { expr: e, .. } => expr(e, lines),
        Expr::Field { object, .. } | Expr::MapField { object, .. } => expr(object, lines),
        Expr::Index { object, index, .. } => {
            expr(object, lines);
            expr(index, lines);
        }
        Expr::Index2 {
            object, row, col, ..
        } => {
            expr(object, lines);
            expr(row, lines);
            expr(col, lines);
        }
        Expr::Faixa { start, end, .. } => {
            expr(start, lines);
            if let Some(e) = end {
                expr(e, lines);
            }
        }
        Expr::Number { .. } | Expr::String { .. } | Expr::Bool { .. } | Expr::Ident { .. } => (),
    }
}

/// A temporary notebook page, deliberately absent from editable documents,
/// recovery, saving and project session persistence.
pub struct Viewer {
    pub page: gtk::Box,
    pub doc: std::rc::Rc<crate::docs::Doc>,
    pub source: Arc<Source>,
    pub edit: gtk::Button,
    pub close: gtk::Button,
    notice: gtk::Label,
}
impl Viewer {
    pub fn new(
        source: Arc<Source>,
        languages: &sourceview5::LanguageManager,
        search: &sourceview5::SearchSettings,
    ) -> Self {
        use gtk::prelude::*;
        let page = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let notice = gtk::Label::new(Some(
            "Fonte em execução — somente leitura. Edições no projeto valem na próxima execução.",
        ));
        notice.set_wrap(true);
        notice.set_xalign(0.0);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let edit = gtk::Button::with_label("Voltar ao editor");
        let close = gtk::Button::with_label("Fechar fonte da execução");
        row.append(&edit);
        row.append(&close);
        let doc = crate::docs::Doc::new(
            Some(source.path.clone()),
            String::new(),
            &source.text,
            languages,
            search,
        );
        doc.view.set_editable(false);
        page.append(&notice);
        page.append(&row);
        page.append(&doc.page);
        Self {
            page,
            doc,
            source,
            edit,
            close,
            notice,
        }
    }
    pub fn finish(&self) {
        self.notice.set_text("Fonte da execução encerrada — somente leitura. Execute novamente para aplicar as edições do projeto.");
        self.doc.clear_debug_line();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk::prelude::*;
    #[test]
    fn executable_locations_include_function_bodies_and_expression_blocks() {
        let source = "importe \"mat\"\n// comment\nf = funcao(x) {\n    retorne se x {\n        x + 1\n    } senao {\n        0\n    }\n}\nescreva(f(1))\n";
        assert_eq!(
            executable_lines(source),
            Some(BTreeSet::from([3, 4, 5, 7, 10]))
        );
        assert_eq!(executable_lines("a = (\n"), None);
    }
    #[test]
    fn source_identity_normalizes_separators_but_keeps_revision_and_content() {
        let source = Source::new("lib\\math.lep", "escreva(1)\n".into());
        assert_eq!(source.path, "lib/math.lep");
        assert!(!source.differs("escreva(1)\n"));
        assert!(source.differs("escreva(2)\n"));
        assert_ne!(
            source.revision,
            Source::new("lib/math.lep", "escreva(2)\n".into()).revision
        );
    }
    #[test]
    fn execution_view_is_read_only_and_separate_from_editable_buffers() {
        crate::gtk_test::run(|| {
            let source = Arc::new(Source::new("lib/b.lep", "escreva(1)\n".into()));
            let languages = sourceview5::LanguageManager::new();
            let search = sourceview5::SearchSettings::new();
            let editor = crate::docs::Doc::new(
                Some(source.path.clone()),
                String::new(),
                &source.text,
                &languages,
                &search,
            );
            let viewer = Viewer::new(Arc::clone(&source), &languages, &search);
            editor.buffer.set_text("escreva(2)\n");
            assert!(!viewer.doc.view.is_editable());
            assert_eq!(viewer.doc.text(), source.text);
            assert_ne!(viewer.doc.buffer, editor.buffer);
            assert!(!viewer.doc.is_dirty());
            viewer.doc.show_debug_line(1, false);
            viewer.finish();
            assert!(viewer.notice.text().contains("encerrada"));
        });
    }
}
