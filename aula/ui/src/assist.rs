//! Language help in the editor: completion (GtkSourceView provider),
//! hover and F1 help, the signature popup while typing arguments, and
//! go to definition. The analysis is in `lang.rs`.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gtk::glib;
use gtk::glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, pango};
use sourceview5::prelude::*;
use sourceview5::subclass::prelude::*;
use sourceview5::{Buffer as SourceBuffer, CompletionColumn, View as SourceView};

use crate::lang::{self, Completion, Def, DefKind, Native};

/// What the editor knows about the project, for help that crosses files.
pub struct Help {
    /// Text of a project file (open tab first, else the server).
    pub file_text: Box<dyn Fn(&str) -> Option<String>>,
    /// Every `.lep` file of the project (relative paths).
    pub project_files: Box<dyn Fn() -> Vec<String>>,
    /// Path of the file a buffer shows (untitled name if not saved).
    pub path_of: Box<dyn Fn(&SourceBuffer) -> Option<String>>,
}

// ── Scope ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum Module {
    Native(&'static str),
    File(String),
}

struct Scope {
    spread: HashSet<&'static str>,
    aliases: HashMap<String, Module>,
    files: Vec<String>,
    defs: Vec<Def>,
}

fn scope(path: &str, text: &str) -> Scope {
    let mut s = Scope {
        spread: HashSet::new(),
        aliases: HashMap::new(),
        files: Vec::new(),
        defs: lang::parse_definitions(text),
    };
    for imp in lang::parse_imports(text) {
        let target = match lang::native_module(&imp.spec) {
            Some(m) => Module::Native(m),
            None => match lang::resolve_import(&imp.spec, path) {
                Some(p) => Module::File(p),
                None => continue,
            },
        };
        match (imp.alias, target) {
            (Some(a), t) => {
                s.aliases.insert(a, t);
            }
            (None, Module::Native(m)) => {
                s.spread.insert(m);
            }
            (None, Module::File(p)) => s.files.push(p),
        }
    }
    s
}

fn file_defs(help: &Help, path: &str) -> Vec<Def> {
    (help.file_text)(path)
        .map(|t| {
            lang::parse_definitions(&t)
                .into_iter()
                .filter(|d| d.top_level)
                .collect()
        })
        .unwrap_or_default()
}

/// Where a name comes from.
pub enum Found {
    Native(&'static Native),
    /// `file: None` = this document.
    Def {
        def: Def,
        file: Option<String>,
    },
}

fn lookup(help: &Help, path: &str, text: &str, alias: Option<&str>, name: &str) -> Option<Found> {
    let sc = scope(path, text);
    if let Some(alias) = alias {
        let target = sc
            .aliases
            .get(alias)
            .cloned()
            .or_else(|| lang::native_module(alias).map(Module::Native))?;
        return match target {
            Module::Native(m) => lang::native(name)
                .filter(|n| n.module == Some(m))
                .map(Found::Native),
            Module::File(f) => file_defs(help, &f)
                .into_iter()
                .find(|d| d.name == name)
                .map(|def| Found::Def { def, file: Some(f) }),
        };
    }
    if let Some(def) = sc.defs.iter().find(|d| d.name == name) {
        return Some(Found::Def {
            def: def.clone(),
            file: None,
        });
    }
    for f in &sc.files {
        if let Some(def) = file_defs(help, f).into_iter().find(|d| d.name == name) {
            return Some(Found::Def {
                def,
                file: Some(f.clone()),
            });
        }
    }
    lang::native(name).map(Found::Native)
}

// ── Completion items ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Function,
    Value,
    Module,
    Keyword,
    File,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub label: String,
    pub kind: Kind,
    /// Right after the name: the parameters, `(texto, frente)`.
    pub params: String,
    /// Where it comes from (module, file), shown dimmed on the right.
    pub detail: String,
    pub doc: String,
    pub insert: String,
    /// Where the cursor goes, in chars from the start of `insert`.
    pub cursor: usize,
    /// `importe "…"` to add if missing.
    pub import: Option<&'static str>,
}

fn call_insert(name: &str, params: usize) -> (String, usize) {
    let n = name.chars().count();
    (format!("{name}()"), if params == 0 { n + 2 } else { n + 1 })
}

fn native_item(n: &'static Native, name: &str, method: bool, import: Option<&'static str>) -> Item {
    let sig = n.signatures.first().cloned().unwrap_or_default();
    let mut params = lang::signature_params(&sig).len();
    if method {
        params = params.saturating_sub(1);
    }
    let (insert, cursor) = if n.is_value {
        (name.to_string(), name.chars().count())
    } else {
        call_insert(name, params)
    };
    let detail = match (n.module, import) {
        (Some(m), Some(_)) => format!("importe \"{m}\" (adiciona)"),
        (Some(m), None) => format!("importe \"{m}\""),
        (None, _) => "nativa".to_string(),
    };
    let params_text = sig
        .find('(')
        .and_then(|a| sig.find(')').map(|z| sig[a..=z].to_string()))
        .unwrap_or_default();
    Item {
        label: name.to_string(),
        kind: if n.is_value {
            Kind::Value
        } else {
            Kind::Function
        },
        params: params_text,
        detail,
        doc: native_doc(n),
        insert,
        cursor,
        import,
    }
}

fn def_item(d: &Def, where_: &str, method: bool) -> Item {
    let (insert, cursor, kind) = match &d.kind {
        DefKind::Function(p) => {
            let n = if method {
                p.len().saturating_sub(1)
            } else {
                p.len()
            };
            let (i, c) = call_insert(&d.name, n);
            (i, c, Kind::Function)
        }
        DefKind::Module => (
            format!("{}::", d.name),
            d.name.chars().count() + 2,
            Kind::Module,
        ),
        DefKind::Value => (d.name.clone(), d.name.chars().count(), Kind::Value),
    };
    Item {
        label: d.name.clone(),
        kind,
        params: match &d.kind {
            DefKind::Function(p) => format!("({})", p.join(", ")),
            _ => String::new(),
        },
        detail: where_.to_string(),
        doc: d.signature(),
        insert,
        cursor,
        import: None,
    }
}

/// Items for the cursor; `before` is the text up to it.
pub fn items(help: &Help, path: &str, text: &str, before: &str) -> (Completion, Vec<Item>) {
    let ctx = lang::completion_context(before);
    let mut out: Vec<Item> = Vec::new();
    let mut seen = HashSet::new();
    let mut push = |out: &mut Vec<Item>, it: Item| {
        if seen.insert(it.label.clone()) {
            out.push(it);
        }
    };
    match &ctx {
        Completion::Nothing => {}
        Completion::Import { .. } => {
            for m in lang::modules() {
                let funcs: Vec<&str> = lang::natives()
                    .iter()
                    .filter(|n| n.module == Some(m))
                    .map(|n| n.name)
                    .collect();
                push(
                    &mut out,
                    Item {
                        label: m.to_string(),
                        kind: Kind::Module,
                        params: String::new(),
                        detail: "módulo nativo".into(),
                        doc: funcs.join(", "),
                        insert: m.to_string(),
                        cursor: m.chars().count(),
                        import: None,
                    },
                );
            }
            let dir = path.rsplit_once('/').map_or("", |(d, _)| d);
            for f in (help.project_files)() {
                if f == path || !f.ends_with(".lep") {
                    continue;
                }
                let Some(rel) = relative_to(dir, &f) else {
                    continue;
                };
                let mut spec = rel.trim_end_matches(".lep").to_string();
                // A bare name that is also a native module needs ./ for the file.
                if lang::native_module(&spec).is_some() {
                    spec = format!("./{spec}");
                }
                push(
                    &mut out,
                    Item {
                        label: spec.clone(),
                        kind: Kind::File,
                        params: String::new(),
                        detail: "arquivo".into(),
                        doc: f.clone(),
                        cursor: spec.chars().count(),
                        insert: spec,
                        import: None,
                    },
                );
            }
        }
        Completion::Module { alias, .. } => {
            let sc = scope(path, text);
            let target = sc
                .aliases
                .get(alias)
                .cloned()
                .or_else(|| lang::native_module(alias).map(Module::Native));
            match target {
                Some(Module::Native(m)) => {
                    for n in lang::natives().iter().filter(|n| n.module == Some(m)) {
                        for name in std::iter::once(n.name).chain(n.aliases.iter().copied()) {
                            push(&mut out, native_item(n, name, false, None));
                        }
                    }
                }
                Some(Module::File(f)) => {
                    for d in file_defs(help, &f) {
                        push(&mut out, def_item(&d, &f, false));
                    }
                }
                None => {}
            }
        }
        Completion::Method { .. } | Completion::Name { .. } => {
            let method = matches!(ctx, Completion::Method { .. });
            let sc = scope(path, text);
            for d in &sc.defs {
                if !method || matches!(d.kind, DefKind::Function(_)) {
                    push(&mut out, def_item(d, "neste arquivo", method));
                }
            }
            for f in &sc.files {
                for d in file_defs(help, f) {
                    if !method || matches!(d.kind, DefKind::Function(_)) {
                        push(&mut out, def_item(&d, f, method));
                    }
                }
            }
            let aliased: HashSet<&str> = sc
                .aliases
                .values()
                .filter_map(|m| match m {
                    Module::Native(m) => Some(*m),
                    Module::File(_) => None,
                })
                .collect();
            for n in lang::natives() {
                let params = n
                    .signatures
                    .first()
                    .map_or(0, |s| lang::signature_params(s).len());
                if method && (n.is_value || params == 0) {
                    continue;
                }
                let import = match n.module {
                    Some(m) if !sc.spread.contains(m) => {
                        // Only imported with an alias: use `alias::name`.
                        if aliased.contains(m) {
                            continue;
                        }
                        Some(m)
                    }
                    _ => None,
                };
                for name in std::iter::once(n.name).chain(n.aliases.iter().copied()) {
                    push(&mut out, native_item(n, name, method, import));
                }
            }
            if !method {
                for alias in sc.aliases.keys() {
                    push(
                        &mut out,
                        Item {
                            label: alias.clone(),
                            kind: Kind::Module,
                            params: String::new(),
                            detail: "módulo".into(),
                            doc: String::new(),
                            insert: format!("{alias}::"),
                            cursor: alias.chars().count() + 2,
                            import: None,
                        },
                    );
                }
                for k in lang::KEYWORDS {
                    push(
                        &mut out,
                        Item {
                            label: k.to_string(),
                            kind: Kind::Keyword,
                            params: String::new(),
                            detail: "palavra-chave".into(),
                            doc: String::new(),
                            insert: k.to_string(),
                            cursor: k.chars().count(),
                            import: None,
                        },
                    );
                }
            }
        }
    }
    (ctx, out)
}

/// `b` relative to directory `dir` (both project-relative); `None` if it
/// would need `..` (the importe would leave the folder).
fn relative_to(dir: &str, b: &str) -> Option<String> {
    if dir.is_empty() {
        return Some(b.to_string());
    }
    b.strip_prefix(&format!("{dir}/")).map(str::to_string)
}

fn matches_prefix(label: &str, prefix: &str) -> bool {
    let fold = |s: &str| -> String {
        s.chars()
            .flat_map(char::to_lowercase)
            .map(fold_accent)
            .collect()
    };
    fold(label).starts_with(&fold(prefix))
}

fn fold_accent(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' => 'a',
        'é' | 'ê' => 'e',
        'í' => 'i',
        'ó' | 'ô' | 'õ' => 'o',
        'ú' | 'ü' => 'u',
        'ç' => 'c',
        c => c,
    }
}

fn prefix_of(ctx: &Completion) -> &str {
    match ctx {
        Completion::Import { prefix }
        | Completion::Module { prefix, .. }
        | Completion::Method { prefix }
        | Completion::Name { prefix } => prefix,
        Completion::Nothing => "",
    }
}

/// GtkSourceView only pops the list when `is_trigger` is true. Identifier
/// letters must count: otherwise typing `esc` never opens completion.
pub fn triggers_completion(before: &str, c: char) -> bool {
    match lang::completion_context(before) {
        Completion::Nothing => false,
        Completion::Import { .. } => c == '"' || c == '/' || lang::is_ident_char(c),
        Completion::Module { .. } => c == ':' || lang::is_ident_char(c),
        Completion::Method { .. } => c == '.' || lang::is_ident_char(c),
        Completion::Name { .. } => lang::is_ident_char(c) || c == '.',
    }
}

/// Prefix the GtkSourceView `refilter` must use: the Expressa token at the
/// cursor (`lib/a` after `importe "`, not the Gtk word `a` after `/`).
fn prefix_before_cursor(buffer: &impl IsA<gtk::TextBuffer>) -> String {
    let buffer = buffer.as_ref();
    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let (s, _) = buffer.bounds();
    let before = buffer.text(&s, &cursor, true).to_string();
    prefix_of(&lang::completion_context(&before)).to_string()
}

// ── Help text ──────────────────────────────────────────────────────────────

fn esc(s: &str) -> String {
    glib::markup_escape_text(s).to_string()
}

pub fn native_doc(n: &Native) -> String {
    let mut s = n.summary.to_string();
    if let Some(m) = n.module {
        s.push_str(&format!("\nMódulo {m}: importe \"{m}\""));
    }
    if !n.aliases.is_empty() {
        s.push_str(&format!("\nTambém: {}", n.aliases.join(", ")));
    }
    if !n.example.is_empty() {
        s.push_str(&format!("\nExemplo: {}", n.example));
    }
    s
}

/// Pango markup for the name at byte `col` of `line` (hover, F1).
pub fn help_markup(help: &Help, path: &str, text: &str, line: &str, col: usize) -> Option<String> {
    let w = lang::word_at(line, col)?;
    if lang::KEYWORDS.contains(&w.name.as_str()) {
        return None;
    }
    Some(
        match lookup(help, path, text, w.alias.as_deref(), &w.name)? {
            Found::Native(n) => {
                let sigs = if n.signatures.is_empty() {
                    n.name.to_string()
                } else {
                    n.signatures.join("\n")
                };
                format!("<tt><b>{}</b></tt>\n{}", esc(&sigs), esc(&native_doc(n)))
            }
            Found::Def { def, file } => {
                let where_ = match file {
                    Some(f) => format!("{f}, linha {}", def.line + 1),
                    None => format!("linha {}", def.line + 1),
                };
                let shown = match &def.kind {
                    DefKind::Function(p) => format!("{} = funcao({})", def.name, p.join(", ")),
                    _ => def.name.clone(),
                };
                let extra = if def.doc.is_empty() {
                    String::new()
                } else {
                    format!("\n{}", esc(&def.doc))
                };
                format!("<tt><b>{}</b></tt>\n{}{}", esc(&shown), esc(&where_), extra)
            }
        },
    )
}

/// Pango markup for the call around the cursor, the current parameter in
/// bold (signature popup).
pub fn signature_markup(help: &Help, path: &str, text: &str, before: &str) -> Option<String> {
    let call = lang::call_context(before)?;
    let active = call.active + usize::from(call.method);
    let (sigs, summary): (Vec<String>, String) =
        match lookup(help, path, text, call.alias.as_deref(), &call.name)? {
            Found::Native(n) if !n.is_value => (n.signatures.clone(), n.summary.to_string()),
            Found::Def {
                def:
                    Def {
                        name,
                        kind: DefKind::Function(p),
                        ..
                    },
                ..
            } => (vec![format!("{name}({})", p.join(", "))], String::new()),
            _ => return None,
        };
    let sig = sigs
        .iter()
        .find(|s| lang::signature_params(s).len() > active || s.contains("..."))
        .or(sigs.first())?;
    let (a, z) = (sig.find('(')?, sig.find(')')?);
    let params = lang::signature_params(sig);
    let shown: Vec<String> = params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if i == active || (p.contains("...") && active >= i) {
                format!("<b><u>{}</u></b>", esc(p))
            } else {
                esc(p)
            }
        })
        .collect();
    let mut m = format!(
        "<tt>{}({}){}</tt>",
        esc(&sig[..a]),
        shown.join(", "),
        esc(&sig[z + 1..])
    );
    if !summary.is_empty() {
        m.push_str(&format!("\n<small>{}</small>", esc(&summary)));
    }
    Some(m)
}

/// Go to definition target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// 1-based line in `file` (`None` = this document).
    Line { file: Option<String>, line: u32 },
    /// The file of an `importe` line.
    File(String),
    /// A native function: show its help.
    Native(&'static str),
}

pub fn definition(help: &Help, path: &str, text: &str, line: &str, col: usize) -> Option<Target> {
    // On `importe "…"`: the file.
    if let Some(imp) = lang::parse_imports(line).first() {
        let start = line.find(&format!("\"{}\"", imp.spec))?;
        if col >= start && col <= start + imp.spec.len() + 2 {
            if lang::native_module(&imp.spec).is_some() {
                return None;
            }
            return lang::resolve_import(&imp.spec, path).map(Target::File);
        }
    }
    let w = lang::word_at(line, col)?;
    match lookup(help, path, text, w.alias.as_deref(), &w.name)? {
        Found::Native(n) => Some(Target::Native(n.name)),
        Found::Def { def, file } => Some(Target::Line {
            file,
            line: def.line as u32 + 1,
        }),
    }
}

// ── GtkSourceView glue ─────────────────────────────────────────────────────

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Proposal {
        pub item: RefCell<Option<Item>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Proposal {
        const NAME: &'static str = "ExpressaProposal";
        type Type = super::Proposal;
        type Interfaces = (sourceview5::CompletionProposal,);
    }
    impl ObjectImpl for Proposal {}
    impl CompletionProposalImpl for Proposal {}

    #[derive(Default)]
    pub struct Provider {
        pub help: RefCell<Option<Rc<Help>>>,
        pub all: RefCell<Vec<super::Proposal>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Provider {
        const NAME: &'static str = "ExpressaProvider";
        type Type = super::Provider;
        type Interfaces = (sourceview5::CompletionProvider,);
    }
    impl ObjectImpl for Provider {}

    impl CompletionProviderImpl for Provider {
        fn title(&self) -> Option<glib::GString> {
            Some("Expressa".into())
        }

        // The binding's defaults call a parent implementation that does not
        // exist for interfaces and panic: answer these ourselves.
        fn priority(&self, _context: &sourceview5::CompletionContext) -> i32 {
            0
        }

        fn list_alternates(
            &self,
            _context: &sourceview5::CompletionContext,
            _proposal: &sourceview5::CompletionProposal,
        ) -> Vec<sourceview5::CompletionProposal> {
            Vec::new()
        }

        fn key_activates(
            &self,
            _context: &sourceview5::CompletionContext,
            _proposal: &sourceview5::CompletionProposal,
            _keyval: gtk::gdk::Key,
            _state: gtk::gdk::ModifierType,
        ) -> bool {
            false
        }

        fn is_trigger(&self, iter: &gtk::TextIter, c: char) -> bool {
            let buffer = iter.buffer();
            let start = buffer.start_iter();
            let before = start.text(iter).to_string();
            super::triggers_completion(&before, c)
        }

        fn populate_future(
            &self,
            context: &sourceview5::CompletionContext,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<gio::ListModel, glib::Error>>>>
        {
            let store = gio::ListStore::new::<super::Proposal>();
            if let (Some(help), Some(buffer)) = (self.help.borrow().clone(), context.buffer()) {
                let path = (help.path_of)(&buffer).unwrap_or_default();
                let (s, e) = buffer.bounds();
                let text = buffer.text(&s, &e, true).to_string();
                let cursor = buffer.iter_at_mark(&buffer.get_insert());
                let before = buffer.text(&s, &cursor, true).to_string();
                let (ctx, list) = super::items(&help, &path, &text, &before);
                let all: Vec<super::Proposal> =
                    list.into_iter().map(super::Proposal::new).collect();
                let prefix = super::prefix_of(&ctx).to_string();
                super::fill_store(&store, &all, &prefix);
                *self.all.borrow_mut() = all;
            }
            Box::pin(std::future::ready(Ok(store.upcast())))
        }

        fn refilter(&self, context: &sourceview5::CompletionContext, model: &gio::ListModel) {
            let Some(store) = model.downcast_ref::<gio::ListStore>() else {
                return;
            };
            let prefix = context
                .buffer()
                .map(|b| super::prefix_before_cursor(&b))
                .unwrap_or_else(|| context.word().to_string());
            super::fill_store(store, &self.all.borrow(), &prefix);
        }

        fn display(
            &self,
            _context: &sourceview5::CompletionContext,
            proposal: &sourceview5::CompletionProposal,
            cell: &sourceview5::CompletionCell,
        ) {
            let Some(item) = proposal
                .downcast_ref::<super::Proposal>()
                .and_then(|p| p.item())
            else {
                return;
            };
            match cell.column() {
                CompletionColumn::Icon => cell.set_text(Some(match item.kind {
                    Kind::Function => "ƒ",
                    Kind::Value => "x",
                    Kind::Module => "▣",
                    Kind::Keyword => "≡",
                    Kind::File => "▤",
                })),
                CompletionColumn::TypedText => cell.set_text(Some(&item.label)),
                CompletionColumn::After => cell.set_text(Some(&item.params)),
                CompletionColumn::Comment => cell.set_text(Some(&item.detail)),
                CompletionColumn::Details => cell.set_text(Some(&item.doc)),
                _ => cell.set_text(None),
            }
        }

        fn activate(
            &self,
            context: &sourceview5::CompletionContext,
            proposal: &sourceview5::CompletionProposal,
        ) {
            let (Some(buffer), Some(item)) = (
                context.buffer(),
                proposal
                    .downcast_ref::<super::Proposal>()
                    .and_then(|p| p.item()),
            ) else {
                return;
            };
            let cursor = buffer.iter_at_mark(&buffer.get_insert());
            let (mut begin, mut end) = context.bounds().unwrap_or((cursor.clone(), cursor.clone()));
            if item.kind == Kind::File
                || item.kind == Kind::Module && {
                    let mut s = cursor.clone();
                    s.set_line_offset(0);
                    s.text(&cursor).contains("importe")
                }
            {
                // Replace the whole spec typed after the quote (`lib/a`).
                let mut s = cursor.clone();
                s.set_line_offset(0);
                let line = s.text(&cursor).to_string();
                if let Some(q) = line.rfind('"') {
                    begin = buffer
                        .iter_at_line_offset(cursor.line(), line[..q + 1].chars().count() as i32)
                        .unwrap_or(begin);
                    end = cursor.clone();
                }
            }
            buffer.begin_user_action();
            buffer.delete(&mut begin, &mut end);
            let start_off = begin.offset();
            buffer.insert(&mut begin, &item.insert);
            buffer.place_cursor(&buffer.iter_at_offset(start_off + item.cursor as i32));
            if let Some(m) = item.import {
                let (s, e) = buffer.bounds();
                let text = buffer.text(&s, &e, true).to_string();
                let already = lang::parse_imports(&text)
                    .iter()
                    .any(|i| i.alias.is_none() && i.spec == m);
                if !already {
                    let line = lang::import_insert_line(&text) as i32;
                    let mut at = buffer
                        .iter_at_line(line)
                        .unwrap_or_else(|| buffer.end_iter());
                    let nl = if at.is_end() && !text.is_empty() && !text.ends_with('\n') {
                        "\n"
                    } else {
                        ""
                    };
                    buffer.insert(&mut at, &format!("{nl}importe \"{m}\"\n"));
                }
            }
            buffer.end_user_action();
        }
    }

    #[derive(Default)]
    pub struct HoverInfo {
        pub help: RefCell<Option<Rc<Help>>>,
        pub diag: RefCell<Option<Box<dyn Fn(&SourceBuffer, i32) -> Option<String>>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HoverInfo {
        const NAME: &'static str = "ExpressaHover";
        type Type = super::HoverInfo;
        type Interfaces = (sourceview5::HoverProvider,);
    }
    impl ObjectImpl for HoverInfo {}

    impl HoverProviderImpl for HoverInfo {
        fn populate_future(
            &self,
            context: &sourceview5::HoverContext,
            display: &sourceview5::HoverDisplay,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), glib::Error>> + 'static>>
        {
            let none = || -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), glib::Error>>>> {
                Box::pin(std::future::ready(Err(glib::Error::new(gio::IOErrorEnum::NotSupported, "sem ajuda"))))
            };
            let buffer = context.buffer();
            let mut iter = buffer.start_iter();
            if !context.is_iter(&mut iter) {
                return none();
            }
            let mut shown = false;
            if let Some(msg) = self
                .diag
                .borrow()
                .as_ref()
                .and_then(|f| f(&buffer, iter.offset()))
            {
                display.append(&super::help_label(&format!(
                    "<span foreground=\"#e53935\"><b>erro:</b></span> {}",
                    esc(&msg)
                )));
                shown = true;
            }
            if let Some(help) = self.help.borrow().clone() {
                let path = (help.path_of)(&buffer).unwrap_or_default();
                let (s, e) = buffer.bounds();
                let text = buffer.text(&s, &e, true).to_string();
                let mut ls = iter.clone();
                ls.set_line_offset(0);
                let mut le = iter.clone();
                if !le.ends_line() {
                    le.forward_to_line_end();
                }
                let line = ls.text(&le).to_string();
                let col = ls.text(&iter).len();
                if let Some(m) = super::help_markup(&help, &path, &text, &line, col) {
                    display.append(&super::help_label(&m));
                    shown = true;
                }
            }
            if shown {
                Box::pin(std::future::ready(Ok(())))
            } else {
                none()
            }
        }
    }
}

glib::wrapper! {
    pub struct Proposal(ObjectSubclass<imp::Proposal>) @implements sourceview5::CompletionProposal;
}

impl Proposal {
    fn new(item: Item) -> Self {
        let p: Self = glib::Object::new();
        *p.imp().item.borrow_mut() = Some(item);
        p
    }

    fn item(&self) -> Option<Item> {
        self.imp().item.borrow().clone()
    }
}

glib::wrapper! {
    pub struct Provider(ObjectSubclass<imp::Provider>) @implements sourceview5::CompletionProvider;
}

glib::wrapper! {
    pub struct HoverInfo(ObjectSubclass<imp::HoverInfo>) @implements sourceview5::HoverProvider;
}

fn help_label(markup: &str) -> gtk::Label {
    let l = gtk::Label::new(None);
    l.set_markup(markup);
    l.set_wrap(true);
    l.set_wrap_mode(pango::WrapMode::WordChar);
    l.set_max_width_chars(70);
    l.set_xalign(0.0);
    l.set_selectable(false);
    l
}

/// Per-view help widgets; `detach` before the view goes away.
pub struct ViewHelp {
    signature: gtk::Popover,
    signature_label: gtk::Label,
    info: gtk::Popover,
    info_label: gtk::Label,
    pending: Rc<Cell<bool>>,
    /// GtkSourceCompletion list is showing: keep our popover down so the
    /// two GtkPopovers do not steal the grab / hide each other.
    completion_open: Cell<bool>,
}

impl ViewHelp {
    pub fn detach(&self) {
        self.signature.unparent();
        self.info.unparent();
    }

    /// Escape: hide signature/F1 popovers and drop the GtkSourceCompletion
    /// context. The list widget hides itself on Escape without cancelling;
    /// a leftover context then never shows the list again.
    pub fn dismiss(&self, view: &SourceView) -> bool {
        let completion = view.completion();
        let had =
            self.completion_open.get() || self.signature.is_visible() || self.info.is_visible();
        self.signature.popdown();
        self.info.popdown();
        completion.hide();
        completion.block_interactive();
        completion.unblock_interactive();
        self.completion_open.set(false);
        had
    }
}

fn fill_store(store: &gio::ListStore, all: &[Proposal], prefix: &str) {
    store.remove_all();
    for p in all {
        if p.item().is_some_and(|i| matches_prefix(&i.label, prefix)) {
            store.append(p);
        }
    }
}

fn cursor_rect(view: &SourceView, dest: &impl IsA<gtk::Widget>) -> gtk::gdk::Rectangle {
    let buffer = view.buffer();
    let it = buffer.iter_at_mark(&buffer.get_insert());
    let r = view.iter_location(&it);
    let (x, y) = view.buffer_to_window_coords(gtk::TextWindowType::Widget, r.x(), r.y());
    let (x, y) = view
        .translate_coordinates(dest, x as f64, y as f64)
        .unwrap_or((x as f64, y as f64));
    gtk::gdk::Rectangle::new(x.round() as i32, y.round() as i32, 1, r.height().max(1))
}

/// Completion, hover, the signature popup and F1 for one editor view.
pub fn attach(
    view: &SourceView,
    help: &Rc<Help>,
    diag: impl Fn(&SourceBuffer, i32) -> Option<String> + 'static,
) -> Rc<ViewHelp> {
    let completion = view.completion();
    let provider: Provider = glib::Object::new();
    *provider.imp().help.borrow_mut() = Some(Rc::clone(help));
    completion.add_provider(&provider);
    completion.set_show_icons(true);
    completion.set_remember_info_visibility(true);
    // First item preselected: Enter takes it (like VS Code).
    completion.set_select_on_show(true);
    // Default page-size is 5; show more names before the list scrolls.
    completion.set_page_size(12);

    let hover = view.hover();
    let info: HoverInfo = glib::Object::new();
    *info.imp().help.borrow_mut() = Some(Rc::clone(help));
    *info.imp().diag.borrow_mut() = Some(Box::new(diag));
    hover.add_provider(&info);
    hover.set_hover_delay(400);

    let signature_label = gtk::Label::new(None);
    signature_label.set_xalign(0.0);
    signature_label.set_wrap(true);
    signature_label.set_wrap_mode(pango::WrapMode::WordChar);
    signature_label.set_max_width_chars(60);
    let signature = gtk::Popover::new();
    signature.set_child(Some(&signature_label));
    signature.set_autohide(false);
    signature.set_has_arrow(false);
    signature.set_can_focus(false);
    signature.set_cascade_popdown(false);
    signature.set_position(gtk::PositionType::Top);
    // Not on the SourceView: GtkSourceCompletion's list is also a popover
    // child of the view, and GTK 4 hides one when the other pops up.
    let host = view.parent().unwrap_or_else(|| view.clone().upcast());
    signature.set_parent(&host);
    signature.add_css_class("aula-signature");

    let info_label = help_label("");
    let info_scroll = gtk::ScrolledWindow::new();
    info_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    info_scroll.set_min_content_width(280);
    info_scroll.set_max_content_width(560);
    info_scroll.set_min_content_height(80);
    info_scroll.set_max_content_height(420);
    info_scroll.set_child(Some(&info_label));
    let info_pop = gtk::Popover::new();
    info_pop.set_child(Some(&info_scroll));
    info_pop.set_can_focus(false);
    info_pop.set_cascade_popdown(false);
    info_pop.set_position(gtk::PositionType::Bottom);
    info_pop.set_parent(&host);

    let vh = Rc::new(ViewHelp {
        signature,
        signature_label,
        info: info_pop,
        info_label,
        pending: Rc::new(Cell::new(false)),
        completion_open: Cell::new(false),
    });
    // Signature popup follows the cursor and the typing (once per idle).
    let update = {
        let weak_vh = Rc::downgrade(&vh);
        let help = Rc::clone(help);
        let view = view.clone();
        move || {
            let Some(vh) = weak_vh.upgrade() else { return };
            if vh.pending.replace(true) {
                return;
            }
            let weak_vh = Rc::downgrade(&vh);
            let help = Rc::clone(&help);
            let view = view.clone();
            glib::idle_add_local_once(move || {
                let Some(vh) = weak_vh.upgrade() else { return };
                vh.pending.set(false);
                if vh.completion_open.get() || !view.has_focus() {
                    vh.signature.popdown();
                    return;
                }
                let buffer = view.buffer();
                let (s, e) = buffer.bounds();
                let cursor = buffer.iter_at_mark(&buffer.get_insert());
                let mut from = cursor.clone();
                from.backward_lines(30);
                from.set_line_offset(0);
                let before = buffer.text(&from, &cursor, true).to_string();
                let text = buffer.text(&s, &e, true).to_string();
                let sb = buffer.downcast_ref::<SourceBuffer>();
                let path = sb.and_then(|b| (help.path_of)(b)).unwrap_or_default();
                match signature_markup(&help, &path, &text, &before) {
                    Some(m) if !buffer.has_selection() => {
                        vh.signature_label.set_markup(&m);
                        // A popover is centered on its target: aim at a box as
                        // wide as the popup so it starts at the cursor.
                        let dest = vh
                            .signature
                            .parent()
                            .unwrap_or_else(|| view.clone().upcast());
                        let r = cursor_rect(&view, &dest);
                        let (_, width, _, _) =
                            vh.signature.measure(gtk::Orientation::Horizontal, -1);
                        let r = gtk::gdk::Rectangle::new(r.x(), r.y(), width.max(1), r.height());
                        vh.signature.set_pointing_to(Some(&r));
                        if !vh.signature.is_visible() {
                            vh.signature.popup();
                        }
                    }
                    _ => vh.signature.popdown(),
                }
            });
        }
    };
    let update = Rc::new(update);
    {
        let u = Rc::clone(&update);
        view.buffer().connect_cursor_position_notify(move |_| u());
    }
    {
        let u = Rc::clone(&update);
        view.buffer().connect_changed(move |_| u());
    }
    {
        let weak_vh = Rc::downgrade(&vh);
        completion.connect_show(move |_| {
            if let Some(vh) = weak_vh.upgrade() {
                vh.completion_open.set(true);
                vh.signature.popdown();
            }
        });
    }
    {
        let weak_vh = Rc::downgrade(&vh);
        let u = Rc::clone(&update);
        completion.connect_hide(move |_| {
            if let Some(vh) = weak_vh.upgrade() {
                vh.completion_open.set(false);
            }
            u();
        });
    }
    {
        let weak_vh = Rc::downgrade(&vh);
        let focus = gtk::EventControllerFocus::new();
        focus.connect_leave(move |_| {
            if let Some(vh) = weak_vh.upgrade() {
                vh.signature.popdown();
            }
        });
        view.add_controller(focus);
    }
    vh
}

/// F1 on a name: ficha popup. Returns false when the cursor is not on a
/// name so the caller can open the Ajuda window.
pub fn show_info(view: &SourceView, vh: &ViewHelp, help: &Help) -> bool {
    let buffer = view.buffer();
    let Some(sb) = buffer.downcast_ref::<SourceBuffer>() else {
        return false;
    };
    let path = (help.path_of)(sb).unwrap_or_default();
    let (s, e) = buffer.bounds();
    let text = buffer.text(&s, &e, true).to_string();
    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let mut ls = cursor.clone();
    ls.set_line_offset(0);
    let mut le = cursor.clone();
    if !le.ends_line() {
        le.forward_to_line_end();
    }
    let line = ls.text(&le).to_string();
    let col = ls.text(&cursor).len();
    let Some(m) = help_markup(help, &path, &text, &line, col) else {
        return false;
    };
    vh.info_label.set_wrap(true);
    vh.info_label.remove_css_class("aula-catalogo");
    vh.info_label.set_markup(&m);
    let dest = vh.info.parent().unwrap_or_else(|| view.clone().upcast());
    let r = cursor_rect(view, &dest);
    let (_, width, _, _) = vh.info.measure(gtk::Orientation::Horizontal, -1);
    vh.info.set_pointing_to(Some(&gtk::gdk::Rectangle::new(
        r.x(),
        r.y(),
        width.max(1),
        r.height(),
    )));
    vh.info.popup();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn help_with(files: &'static [(&'static str, &'static str)]) -> Help {
        Help {
            file_text: Box::new(move |p| {
                files
                    .iter()
                    .find(|(n, _)| *n == p)
                    .map(|(_, t)| t.to_string())
            }),
            project_files: Box::new(move || files.iter().map(|(n, _)| n.to_string()).collect()),
            path_of: Box::new(|_| None),
        }
    }

    const FILES: &[(&str, &str)] = &[
        (
            "lib/ajuda.lep",
            "dobro = funcao(x) {\n    x * 2\n}\ninterno = 1\n",
        ),
        ("main.lep", ""),
        ("matriz.lep", "x = 1\n"),
    ];

    fn labels(items: &[Item]) -> Vec<&str> {
        items.iter().map(|i| i.label.as_str()).collect()
    }

    #[test]
    fn completion_items() {
        let h = help_with(FILES);
        let text = "importe \"lib/ajuda\"\nm = importe \"mat\"\nsoma = funcao(a, b) { a + b }\n";
        let (_, it) = items(&h, "main.lep", text, "pi");
        let pinte = it.iter().find(|i| i.label == "pinte").unwrap();
        assert_eq!(
            pinte.import,
            Some("tela"),
            "module function adds its importe"
        );
        assert_eq!((pinte.insert.as_str(), pinte.cursor), ("pinte()", 6));
        let l = labels(&it);
        assert!(l.contains(&"soma") && l.contains(&"dobro") && l.contains(&"interno"));
        assert!(
            !l.contains(&"raiz"),
            "mat is only imported with an alias: m::raiz"
        );
        assert!(l.contains(&"se_falhar") && l.contains(&"m"));
        let cls = it.iter().find(|i| i.label == "cls").unwrap();
        assert_eq!(cls.cursor, 5, "no parameters: cursor after ()");

        let (_, it) = items(&h, "main.lep", text, "x = m::");
        let l = labels(&it);
        assert!(
            l.contains(&"raiz")
                && l.contains(&"semente")
                && l.contains(&"seno")
                && l.contains(&"pi"),
            "{l:?}"
        );
        let (_, it) = items(&h, "main.lep", text, "lista.");
        let l = labels(&it);
        assert!(
            l.contains(&"tamanho")
                && l.contains(&"soma")
                && !l.contains(&"cls")
                && !l.contains(&"se")
        );
        let t = it.iter().find(|i| i.label == "tamanho").unwrap();
        assert_eq!(
            t.cursor,
            "tamanho()".len(),
            "the receiver is the only parameter"
        );

        let (_, it) = items(&h, "main.lep", text, "importe \"");
        let l = labels(&it);
        assert!(l.contains(&"tela") && l.contains(&"lib/ajuda") && l.contains(&"./matriz"));
        assert!(!l.contains(&"main"), "not the file itself");
        let (_, it) = items(&h, "lib/x.lep", "", "importe \"");
        assert!(
            labels(&it).contains(&"ajuda"),
            "relative to the file's folder"
        );
        assert!(items(&h, "main.lep", text, "x = \"te").1.is_empty());

        let (_, it) = items(&h, "main.lep", text, r#"importe "lib/a"#);
        assert!(labels(&it).contains(&"lib/ajuda"));
    }

    #[test]
    fn letters_trigger_name_completion() {
        assert!(triggers_completion("esc", 'c'));
        assert!(triggers_completion("escreva", 'a'));
        assert!(triggers_completion("xs.", '.'));
        assert!(triggers_completion("m::", ':'));
        assert!(triggers_completion(r#"importe "te"#, 'e'));
        assert!(!triggers_completion("x = 3", '3'));
        assert!(!triggers_completion("x // esc", 'c'));
        assert!(!triggers_completion("x = \"te", 'e'));
    }

    #[test]
    fn prefix_ignores_case_and_accents() {
        assert!(matches_prefix("aleatório", "aleato"));
        assert!(matches_prefix("função", "func"));
        assert!(matches_prefix("Escreva", "esc"));
        assert!(!matches_prefix("escreva", "x"));
        // GtkSourceView's word after `/` is just `a`; we keep the spec prefix.
        assert!(matches_prefix("lib/ajuda", "lib/a"));
        assert!(!matches_prefix("lib/ajuda", "a"));
        let ctx = lang::completion_context(r#"importe "lib/a"#);
        assert_eq!(prefix_of(&ctx), "lib/a");
        assert!(matches_prefix("lib/ajuda", prefix_of(&ctx)));
    }

    #[test]
    fn signature_and_help() {
        let h = help_with(FILES);
        let text = "importe \"tela\"\nsoma = funcao(a, b) { a + b }\n";
        let m = signature_markup(&h, "main.lep", text, "pinte(\"a\", ").unwrap();
        assert!(m.contains("<b><u>frente</u></b>"), "{m}");
        let m = signature_markup(&h, "main.lep", text, "\"a\".pinte(:verde, ").unwrap();
        assert!(
            m.contains("<b><u>fundo</u></b>"),
            "method call shifts by one: {m}"
        );
        let m = signature_markup(&h, "main.lep", text, "soma(1, ").unwrap();
        assert!(m.contains("<b><u>b</u></b>"), "{m}");
        let m = signature_markup(&h, "main.lep", text, "escreva(1, 2, ").unwrap();
        assert!(m.contains("<u>...</u>"), "variadic: {m}");
        assert_eq!(signature_markup(&h, "main.lep", text, "x = 1"), None);

        let m = help_markup(&h, "main.lep", text, "escreva(soma(1, 2))", 2).unwrap();
        assert!(m.contains("escreva(valor, ...)"), "{m}");
        let m = help_markup(&h, "main.lep", text, "escreva(soma(1, 2))", 10).unwrap();
        assert!(
            m.contains("soma = funcao(a, b)") && m.contains("linha 2"),
            "{m}"
        );
        assert_eq!(help_markup(&h, "main.lep", text, "se x {", 1), None);
        assert_eq!(help_markup(&h, "main.lep", text, "    ", 2), None);
        assert_eq!(help_markup(&h, "main.lep", text, "", 0), None);
    }

    #[test]
    fn go_to_definition() {
        let h = help_with(FILES);
        let text =
            "importe \"lib/ajuda\"\nsoma = funcao(a, b) { a + b }\nescreva(soma(dobro(1), 2))\n";
        assert_eq!(
            definition(&h, "main.lep", text, "escreva(soma(dobro(1), 2))", 9),
            Some(Target::Line {
                file: None,
                line: 2
            })
        );
        assert_eq!(
            definition(&h, "main.lep", text, "escreva(soma(dobro(1), 2))", 14),
            Some(Target::Line {
                file: Some("lib/ajuda.lep".into()),
                line: 1
            })
        );
        assert_eq!(
            definition(&h, "main.lep", text, "importe \"lib/ajuda\"", 12),
            Some(Target::File("lib/ajuda.lep".into()))
        );
        assert_eq!(
            definition(&h, "main.lep", text, "escreva(1)", 2),
            Some(Target::Native("escreva"))
        );
        assert_eq!(
            definition(&h, "main.lep", text, "importe \"tela\"", 10),
            None
        );
    }
}
