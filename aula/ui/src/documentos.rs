//! Document lifecycle policy, independent of the application window.
//!
//! The UI presents dialogs and attaches tabs; this controller decides whether
//! to reuse/read a document and how to save it. Its inputs are a short-lived
//! snapshot, so buffer notifications never run under a borrow of the whole UI.

use std::rc::Rc;

use crate::docs::Doc;
use crate::exec::same_project_path;
use crate::recuperacao::{Copy, UNTITLED_PREFIX};
use crate::store::Files;
use gtk::prelude::*;

pub enum OpenDocument {
    Existing(Rc<Doc>),
    Loaded { path: String, text: String },
}

#[derive(Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    Saved {
        name: String,
        recovery_warning: Option<String>,
    },
    Cancelled,
}

pub struct RecoveredDocument {
    pub doc: Rc<Doc>,
    /// Missing originals and conflicting open buffers are recovered into new tabs.
    pub warning: Option<String>,
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

    pub fn recover(
        &self,
        copy: &Copy,
        create: impl FnOnce(Option<String>, &str) -> Result<Rc<Doc>, String>,
    ) -> Result<RecoveredDocument, String> {
        let (doc, warning) = if copy.backup.name.starts_with(UNTITLED_PREFIX) {
            (create(None, &copy.text)?, None)
        } else {
            match self.open(&copy.backup.name) {
                Ok(OpenDocument::Existing(doc)) if !doc.is_dirty() => (doc, None),
                Ok(OpenDocument::Loaded { path, text }) => (create(Some(path), &text)?, None),
                Ok(OpenDocument::Existing(_)) => (
                    create(None, &copy.text)?,
                    Some("a aba original tem alterações não salvas".to_string()),
                ),
                Err(e) => (create(None, &copy.text)?, Some(e)),
            }
        };
        if doc.text() != copy.text {
            let line = doc.cursor().line();
            doc.buffer.begin_user_action();
            doc.buffer.set_text(&copy.text);
            doc.buffer.end_user_action();
            if let Some(iter) = doc.buffer.iter_at_line(line) {
                doc.buffer.place_cursor(&iter);
            }
        }
        *doc.copia_recuperacao.borrow_mut() = Some(copy.backup.clone());
        // Loading identical text, especially into a new untitled tab, does not
        // make GTK mark it dirty. Every recovered copy requires acknowledgement.
        doc.buffer.set_modified(true);
        doc.refresh_label();
        Ok(RecoveredDocument { doc, warning })
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
        let recovery_warning = forget_recovery(doc).err();
        Ok(SaveOutcome::Saved {
            name,
            recovery_warning,
        })
    }
}

/// A successful save or an explicitly accepted close owns only this tab's copies.
/// A cleanup failure keeps its metadata available for retry.
pub fn forget_recovery(doc: &Doc) -> Result<(), String> {
    if let Some(backup) = doc.copia_recuperacao.borrow().as_ref() {
        backup.remove()?;
    }
    doc.copia_recuperacao.borrow_mut().take();
    doc.resumo_da_copia.set(None);
    doc.refresh_label();
    Ok(())
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
                Ok(SaveOutcome::Saved {
                    name: "novo.lep".into(),
                    recovery_warning: None
                })
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
                Ok(SaveOutcome::Saved {
                    name: "novo.lep".into(),
                    recovery_warning: None
                })
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

    #[test]
    fn recovered_untitled_work_survives_cancel_restart_and_failed_save() {
        crate::gtk_test::run(|| {
            let project = Project::new();
            let store = crate::recuperacao::RecoveryStore(project.0.join("recovery"));
            store
                .save("sem-titulo:lição.lep", "escreva(42)\n", None)
                .unwrap();
            let files = Files::None;
            let documents = Documents::new(&files, "", &[]);
            let copy = store.scan().copies.pop().unwrap();
            let restored = documents
                .recover(&copy, |path, text| Ok(doc(path.as_deref(), text)))
                .unwrap();
            assert!(restored.doc.is_untitled());
            assert!(restored.doc.is_dirty());
            assert_eq!(close_decision(&restored.doc), CloseDecision::ConfirmUnsaved);
            assert!(!resolve_unsaved(
                UnsavedChoice::Cancel,
                std::slice::from_ref(&restored.doc),
                |_| panic!("cancel must not save")
            ));
            assert!(documents.save(&restored.doc, Some("lição.lep")).is_err());
            assert!(restored.doc.is_dirty());
            assert_eq!(store.scan().copies[0].text, copy.text);
            // Simulate losing the in-memory tab immediately after recovery.
            drop(restored);
            let restarted = crate::recuperacao::RecoveryStore(store.0.clone()).scan();
            assert_eq!(restarted.copies.len(), 1);
            let restored = documents
                .recover(&restarted.copies[0], |path, text| {
                    Ok(doc(path.as_deref(), text))
                })
                .unwrap();
            let local = Files::Local(project.0.clone());
            let saved = Documents::new(&local, "local", &[])
                .save(&restored.doc, Some("lição.lep"))
                .unwrap();
            assert_eq!(
                saved,
                SaveOutcome::Saved {
                    name: "lição.lep".into(),
                    recovery_warning: None
                }
            );
            assert_eq!(
                std::fs::read_to_string(project.0.join("lição.lep")).unwrap(),
                copy.text
            );
            assert!(!restored.doc.is_dirty());
            assert!(store.scan().copies.is_empty());
        });
    }

    #[test]
    fn missing_original_and_failed_recovery_do_not_erase_other_copies() {
        crate::gtk_test::run(|| {
            let project = Project::new();
            let store = crate::recuperacao::RecoveryStore(project.0.join("recovery"));
            store.save("missing.lep", "escreva(1)\n", None).unwrap();
            store.save("other.lep", "escreva(2)\n", None).unwrap();
            let scan = store.scan();
            let copy = scan
                .copies
                .iter()
                .find(|c| c.backup.name == "missing.lep")
                .unwrap();
            let other = scan
                .copies
                .iter()
                .find(|c| c.backup.name == "other.lep")
                .unwrap();
            let files = Files::Local(project.0.clone());
            let documents = Documents::new(&files, "local", &[]);
            assert!(
                documents
                    .recover(other, |_, _| Err("could not create tab".into()))
                    .is_err()
            );
            let restored = documents
                .recover(copy, |path, text| Ok(doc(path.as_deref(), text)))
                .unwrap();
            assert!(restored.warning.is_some());
            assert!(restored.doc.is_untitled());
            assert!(restored.doc.is_dirty());
            assert_eq!(
                restored
                    .doc
                    .copia_recuperacao
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .name,
                "missing.lep"
            );
            assert!(
                restored
                    .doc
                    .tab
                    .tooltip_text()
                    .unwrap()
                    .contains("missing.lep")
            );
            assert_eq!(store.scan().copies.len(), 2);
            // Explicit discard affects only the recovered tab; failures remain.
            assert!(resolve_unsaved(
                UnsavedChoice::Discard,
                std::slice::from_ref(&restored.doc),
                |_| panic!("discard must not save")
            ));
            forget_recovery(&restored.doc).unwrap();
            let remaining = store.scan();
            assert_eq!(remaining.copies.len(), 1);
            assert_eq!(remaining.copies[0].backup.id, other.backup.id);
        });
    }

    #[test]
    fn recovery_preserves_dirty_original_and_marks_identical_text_unsaved() {
        crate::gtk_test::run(|| {
            let project = Project::new();
            let store = crate::recuperacao::RecoveryStore(project.0.join("recovery"));
            store.save("a.lep", "escreva(7)\n", None).unwrap();
            let copy = store.scan().copies.pop().unwrap();
            let existing = doc(Some("a.lep"), "escreva(99)\n");
            existing.buffer.set_modified(true);
            let files = Files::Local(project.0.clone());
            let docs = [existing.clone()];
            let documents = Documents::new(&files, "local", &docs);
            let detached = documents
                .recover(&copy, |path, text| Ok(doc(path.as_deref(), text)))
                .unwrap();
            assert!(detached.doc.is_untitled());
            assert!(detached.warning.is_some());
            assert_eq!(existing.text(), "escreva(99)\n");
            assert!(existing.is_dirty());
            drop(detached);
            // Even identical saved/original text requires an explicit decision.
            existing.buffer.set_text(&copy.text);
            existing.mark_saved();
            let restored = documents
                .recover(&copy, |_, _| panic!("reuse the clean original tab"))
                .unwrap();
            assert!(Rc::ptr_eq(&restored.doc, &existing));
            assert!(existing.is_dirty());
            assert!(copy.backup.paths[0].exists());
        });
    }
}
