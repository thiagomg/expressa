//! Document lifecycle policy, independent of the application window.
//!
//! The UI presents dialogs and attaches tabs; this controller decides whether
//! to reuse/read a document and how to save it. Its inputs are a short-lived
//! snapshot, so buffer notifications never run under a borrow of the whole UI.

use std::rc::Rc;

use crate::docs::Doc;
use crate::exec::same_project_path;
use crate::store::Files;

pub enum OpenDocument {
    Existing(Rc<Doc>),
    Loaded { path: String, text: String },
}

#[derive(Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    Saved(String),
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseDecision {
    Close,
    ConfirmUnsaved,
}

#[derive(Debug, Clone, Copy)]
pub enum UnsavedChoice {
    Save,
    Discard,
    Cancel,
}

pub struct Documents<'a> {
    files: &'a Files,
    project: &'a str,
    docs: &'a [Rc<Doc>],
}

impl<'a> Documents<'a> {
    pub fn new(files: &'a Files, project: &'a str, docs: &'a [Rc<Doc>]) -> Self {
        Self {
            files,
            project,
            docs,
        }
    }

    pub fn open(&self, path: &str) -> Result<OpenDocument, String> {
        if let Some(doc) = find_by_name(self.docs, path) {
            return Ok(OpenDocument::Existing(doc));
        }
        let text = self.files.read_file(self.project, path)?;
        Ok(OpenDocument::Loaded {
            path: path.to_string(),
            text,
        })
    }

    /// `new_name` is the answer to the UI's save-as prompt, if needed.
    /// A cancelled or failed save leaves both the path and dirty state intact.
    pub fn save(&self, doc: &Doc, new_name: Option<&str>) -> Result<SaveOutcome, String> {
        let name = if doc.is_untitled() {
            let Some(name) = new_name else {
                return Ok(SaveOutcome::Cancelled);
            };
            let name = name.trim().trim_start_matches('/');
            if name.is_empty() {
                return Ok(SaveOutcome::Cancelled);
            }
            let name = if name.contains('.') {
                name.to_string()
            } else {
                format!("{name}.lep")
            };
            if find_by_name(self.docs, &name).is_some() {
                return Err(format!("`{name}` já está aberto em outra aba"));
            }
            self.files
                .create_file(self.project, &name, &doc.text())
                .map_err(|e| format!("erro ao salvar {name}: {e}"))?;
            doc.set_path(name.clone());
            name
        } else {
            let name = doc.name();
            self.files
                .write_file(self.project, &name, &doc.text())
                .map_err(|e| format!("erro ao salvar {name}: {e}"))?;
            name
        };
        doc.mark_saved();
        Ok(SaveOutcome::Saved(name))
    }
}

pub fn find_by_name(docs: &[Rc<Doc>], name: &str) -> Option<Rc<Doc>> {
    docs.iter()
        .find(|d| same_project_path(&d.name(), name))
        .cloned()
}

pub fn close_decision(doc: &Doc) -> CloseDecision {
    if doc.is_dirty() {
        CloseDecision::ConfirmUnsaved
    } else {
        CloseDecision::Close
    }
}

/// Resolve a UI choice before closing tabs or switching projects. Saving stops
/// at the first failure/cancellation; only the caller performs the actual close.
pub fn resolve_unsaved(
    choice: UnsavedChoice,
    docs: &[Rc<Doc>],
    save: impl FnMut(&Rc<Doc>) -> bool,
) -> bool {
    match choice {
        UnsavedChoice::Save => docs.iter().all(save),
        UnsavedChoice::Discard => true,
        UnsavedChoice::Cancel => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk::prelude::*;
    use std::path::PathBuf;

    struct Project(PathBuf);

    impl Project {
        fn new() -> Self {
            // One GTK test owns this fixture; parallel processes use different paths.
            let path =
                std::env::temp_dir().join(format!("expressa-documents-{}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Project {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn doc(path: Option<&str>, text: &str) -> Rc<Doc> {
        Doc::new(
            path.map(str::to_string),
            "sem-titulo.lep".into(),
            text,
            &sourceview5::LanguageManager::new(),
            &sourceview5::SearchSettings::new(),
        )
    }

    #[test]
    fn lifecycle_without_application_window() {
        crate::gtk_test::run(|| {
            let project = Project::new();
            let files = Files::Local(project.0.clone());
            std::fs::write(project.0.join("existing.lep"), "escreva(1)\n").unwrap();
            let open = doc(Some("lib/aberto.lep"), "escreva(2)\n");
            let untitled = doc(None, "escreva(3)\n");
            untitled.buffer.set_modified(true);
            let docs = [open.clone(), untitled.clone()];
            let controller = Documents::new(&files, "local", &docs);

            // An already-open, even unsaved document wins over a disk read.
            let OpenDocument::Existing(found) = controller.open("lib\\aberto.lep").unwrap() else {
                panic!("should reuse the open tab");
            };
            assert!(Rc::ptr_eq(&open, &found));
            let OpenDocument::Loaded { path, text } = controller.open("existing.lep").unwrap()
            else {
                panic!("should read the unopened file");
            };
            assert_eq!(path, "existing.lep");
            assert_eq!(text, "escreva(1)\n");
            assert!(controller.open("missing.lep").is_err());

            assert_eq!(close_decision(&untitled), CloseDecision::ConfirmUnsaved);
            for name in [None, Some("  /  ")] {
                assert_eq!(controller.save(&untitled, name), Ok(SaveOutcome::Cancelled));
                assert!(untitled.is_untitled());
                assert!(untitled.is_dirty());
            }
            assert!(controller.save(&untitled, Some("lib/aberto.lep")).is_err());
            assert!(controller.save(&untitled, Some("existing.lep")).is_err());
            assert!(controller.save(&untitled, Some("../outside.lep")).is_err());
            assert!(untitled.is_untitled());
            assert!(untitled.is_dirty());
            assert_eq!(
                std::fs::read_to_string(project.0.join("existing.lep")).unwrap(),
                "escreva(1)\n"
            );

            assert_eq!(
                controller.save(&untitled, Some(" /novo ")),
                Ok(SaveOutcome::Saved("novo.lep".into()))
            );
            assert_eq!(untitled.name(), "novo.lep");
            assert_eq!(close_decision(&untitled), CloseDecision::Close);
            assert_eq!(
                std::fs::read_to_string(project.0.join("novo.lep")).unwrap(),
                untitled.text()
            );

            untitled.buffer.set_text("escreva(4)\n");
            assert!(untitled.is_dirty());
            assert_eq!(
                controller.save(&untitled, None),
                Ok(SaveOutcome::Saved("novo.lep".into()))
            );
            assert_eq!(
                std::fs::read_to_string(project.0.join("novo.lep")).unwrap(),
                "escreva(4)\n"
            );

            // A failed named save also leaves its document unsaved.
            let no_project = Files::None;
            untitled.buffer.set_modified(true);
            assert!(
                Documents::new(&no_project, "", &docs)
                    .save(&untitled, None)
                    .is_err()
            );
            assert!(untitled.is_dirty());

            let mut attempts = Vec::new();
            for choice in [UnsavedChoice::Cancel, UnsavedChoice::Discard] {
                let proceed = resolve_unsaved(choice, &docs, |d| {
                    attempts.push(d.name());
                    true
                });
                assert_eq!(proceed, matches!(choice, UnsavedChoice::Discard));
                assert!(attempts.is_empty());
            }
            assert!(!resolve_unsaved(UnsavedChoice::Save, &docs, |d| {
                attempts.push(d.name());
                false
            }));
            assert_eq!(attempts, [open.name()]);
            attempts.clear();
            assert!(resolve_unsaved(UnsavedChoice::Save, &docs, |d| {
                attempts.push(d.name());
                true
            }));
            assert_eq!(attempts, [open.name(), untitled.name()]);
        });
    }
}
