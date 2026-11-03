//! Document lifecycle policy, independent of the application window.
//!
//! The UI presents dialogs and attaches tabs; this controller decides whether
//! to reuse/read a document and how to save it. Its inputs are a short-lived
//! snapshot, so buffer notifications never run under a borrow of the whole UI.

use std::rc::Rc;

use crate::docs::Doc;
use crate::exec::same_project_path;
use crate::recuperacao::{Copy, UNTITLED_PREFIX};
use crate::store::{Expected, Files, SaveError};
use gtk::prelude::*;

pub enum OpenDocument {
    Existing(Rc<Doc>),
    Loaded {
        path: String,
        text: String,
        revision: Option<String>,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    Saved {
        name: String,
        warning: Option<String>,
    },
    Cancelled,
    Conflict {
        name: String,
    },
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
    #[cfg(test)]
    files: &'a Files,
    #[cfg(test)]
    project: &'a str,
    docs: &'a [Rc<Doc>],
}

impl<'a> Documents<'a> {
    pub fn new(files: &'a Files, project: &'a str, docs: &'a [Rc<Doc>]) -> Self {
        let _ = (files, project);
        Self {
            #[cfg(test)]
            files,
            #[cfg(test)]
            project,
            docs,
        }
    }

    #[cfg(test)]
    pub fn open(&self, path: &str) -> Result<OpenDocument, String> {
        if let Some(doc) = find_by_name(self.docs, path) {
            return Ok(OpenDocument::Existing(doc));
        }
        let snapshot = self
            .files
            .current_text(self.project, path)?
            .ok_or_else(|| format!("arquivo `{path}` não existe"))?;
        Ok(OpenDocument::Loaded {
            path: path.to_string(),
            text: snapshot.text,
            revision: snapshot.revision,
        })
    }

    #[cfg(test)]
    pub fn recover(
        &self,
        copy: &Copy,
        create: impl FnOnce(Option<String>, &str) -> Result<Rc<Doc>, String>,
    ) -> Result<RecoveredDocument, String> {
        let original = self.open(&copy.backup.name);
        self.apply_recovery(copy, original, create)
    }

    pub fn recover_loaded(
        &self,
        copy: &Copy,
        snapshot: Result<Option<crate::store::TextSnapshot>, String>,
        create: impl FnOnce(Option<String>, &str) -> Result<Rc<Doc>, String>,
    ) -> Result<RecoveredDocument, String> {
        let original = if let Some(doc) = find_by_name(self.docs, &copy.backup.name) {
            Ok(OpenDocument::Existing(doc))
        } else {
            snapshot
                .and_then(|s| s.ok_or_else(|| "o arquivo original não existe".into()))
                .map(|s| OpenDocument::Loaded {
                    path: copy.backup.name.clone(),
                    text: s.text,
                    revision: s.revision,
                })
        };
        self.apply_recovery(copy, original, create)
    }

    fn apply_recovery(
        &self,
        copy: &Copy,
        original: Result<OpenDocument, String>,
        create: impl FnOnce(Option<String>, &str) -> Result<Rc<Doc>, String>,
    ) -> Result<RecoveredDocument, String> {
        let (doc, warning) = if copy.backup.name.starts_with(UNTITLED_PREFIX) {
            (create(None, &copy.text)?, None)
        } else {
            match original {
                Ok(OpenDocument::Existing(doc)) if !doc.is_dirty() => (doc, None),
                Ok(OpenDocument::Loaded {
                    path,
                    text,
                    revision,
                }) => {
                    let doc = create(Some(path), &text)?;
                    *doc.revision.borrow_mut() = revision;
                    (doc, None)
                }
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

    /// A cancelled/conflicting/failed save keeps buffer contents and recovery copies.
    #[cfg(test)]
    pub fn save(&self, doc: &Doc, new_name: Option<&str>) -> Result<SaveOutcome, String> {
        self.save_to(doc, new_name, doc.is_untitled(), None)
    }

    #[cfg(test)]
    pub fn save_as(&self, doc: &Doc, name: &str) -> Result<SaveOutcome, String> {
        self.save_to(doc, Some(name), true, None)
    }

    /// Explicit overwrite is still conditional on the revision shown to the user.
    #[cfg(test)]
    pub fn overwrite(&self, doc: &Doc, expected: &Expected) -> Result<SaveOutcome, String> {
        self.save_to(doc, None, false, Some(expected))
    }

    #[cfg(test)]
    pub fn reload(&self, doc: &Doc) -> Result<Option<String>, String> {
        let snapshot = self
            .files
            .current_text(self.project, &doc.name())?
            .ok_or("o arquivo foi removido; a aba foi preservada")?;
        doc.recarregar(&snapshot.text);
        *doc.revision.borrow_mut() = snapshot.revision;
        Ok(forget_recovery(doc).err())
    }

    pub fn prepare_save(
        &self,
        doc: &Doc,
        new_name: Option<&str>,
        creating: bool,
        explicit: Option<&Expected>,
    ) -> Result<Option<SaveRequest>, String> {
        let name = if creating {
            let Some(name) = new_name else {
                return Ok(None);
            };
            let name = name.trim().trim_start_matches('/');
            if name.is_empty() {
                return Ok(None);
            }
            let name = if name.contains('.') {
                name.to_string()
            } else {
                format!("{name}.lep")
            };
            if self.docs.iter().any(|other| {
                !std::ptr::eq(other.as_ref(), doc) && same_project_path(&other.name(), &name)
            }) {
                return Err(format!("`{name}` já está aberto em outra aba"));
            }
            name
        } else {
            doc.name()
        };
        let expected = if creating {
            Expected::Missing
        } else {
            explicit.cloned().unwrap_or_else(|| {
                doc.revision
                    .borrow()
                    .clone()
                    .map(Expected::Revision)
                    .unwrap_or(Expected::Unversioned)
            })
        };
        Ok(Some(SaveRequest {
            name,
            text: doc.text(),
            expected,
            creating,
            original_name: doc.name(),
            edit_epoch: doc.edit_epoch.get(),
            io_epoch: doc.io_epoch.get(),
        }))
    }

    #[cfg(test)]
    fn save_to(
        &self,
        doc: &Doc,
        new_name: Option<&str>,
        creating: bool,
        explicit: Option<&Expected>,
    ) -> Result<SaveOutcome, String> {
        let Some(request) = self.prepare_save(doc, new_name, creating, explicit)? else {
            return Ok(SaveOutcome::Cancelled);
        };
        let result = request.write(self.files, self.project);
        let mut outcome = request.complete(doc, result)?;
        if let SaveOutcome::Saved { warning, .. } = &mut outcome
            && !doc.is_dirty()
            && let Err(e) = forget_recovery(doc)
        {
            *warning = Some(
                warning
                    .take()
                    .map_or_else(|| e.clone(), |w| format!("{w}\n{e}")),
            );
        }
        Ok(outcome)
    }
}

/// Immutable GTK snapshot: only these bytes and this expected storage revision
/// may be written by the worker. Completion never acknowledges newer edits.
#[derive(Clone)]
pub struct SaveRequest {
    pub name: String,
    pub text: String,
    pub expected: Expected,
    pub creating: bool,
    pub original_name: String,
    pub edit_epoch: u64,
    pub io_epoch: u64,
}
impl SaveRequest {
    pub fn write(&self, files: &Files, project: &str) -> Result<crate::store::Saved, SaveError> {
        files.write_file(project, &self.name, &self.text, &self.expected)
    }
    pub fn complete(
        &self,
        doc: &Doc,
        result: Result<crate::store::Saved, SaveError>,
    ) -> Result<SaveOutcome, String> {
        if doc.name() != self.original_name || doc.io_epoch.get() != self.io_epoch {
            return Err("a aba mudou de nome durante a gravação; confira o arquivo salvo".into());
        }
        let receipt = match result {
            Ok(receipt) => receipt,
            Err(SaveError::Conflict) => {
                doc.buffer.set_modified(true);
                return Ok(SaveOutcome::Conflict {
                    name: self.name.clone(),
                });
            }
            Err(SaveError::AlreadyExists) if !self.creating => {
                doc.buffer.set_modified(true);
                return Ok(SaveOutcome::Conflict {
                    name: self.name.clone(),
                });
            }
            Err(e) => return Err(format!("erro ao salvar {}: {e}", self.name)),
        };
        if self.creating {
            doc.vigia.replace(None);
            doc.set_path(self.name.clone());
        }
        *doc.revision.borrow_mut() = Some(receipt.revision);
        doc.recovery.saved(self.edit_epoch);
        if doc.edit_epoch.get() == self.edit_epoch {
            doc.mark_saved();
        } else {
            doc.buffer.set_modified(true);
        }
        Ok(SaveOutcome::Saved {
            name: self.name.clone(),
            warning: receipt.warning,
        })
    }
}

/// A successful save or an explicitly accepted close owns only this tab's copies.
/// A cleanup failure keeps its metadata available for retry.
#[cfg(test)]
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
#[cfg(test)]
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
        let doc = Doc::new(
            path.map(str::to_string),
            "sem-titulo.lep".into(),
            text,
            &sourceview5::LanguageManager::new(),
            &sourceview5::SearchSettings::new(),
        );
        if path.is_some() {
            *doc.revision.borrow_mut() = Some(expressa_aula_storage::revision(text.as_bytes()));
        }
        doc
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
            let OpenDocument::Loaded { path, text, .. } = controller.open("existing.lep").unwrap()
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
                    warning: None
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
                    warning: None
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
    fn conflicting_save_retains_buffer_backup_and_requires_fresh_overwrite_consent() {
        crate::gtk_test::run(|| {
            let project = Project::new();
            let files = Files::Local(project.0.clone());
            let path = project.0.join("a.lep");
            std::fs::write(&path, "base").unwrap();
            let edited = doc(Some("a.lep"), "base");
            edited.buffer.set_text("mine");
            let store = crate::recuperacao::RecoveryStore(project.0.join("recovery"));
            let backup = store.save("a.lep", "mine", None).unwrap();
            *edited.copia_recuperacao.borrow_mut() = Some(backup.clone());
            let docs = [edited.clone()];
            let controller = Documents::new(&files, "local", &docs);
            std::fs::write(&path, "disk").unwrap(); // Same length as the base.
            assert_eq!(
                controller.save(&edited, None).unwrap(),
                SaveOutcome::Conflict {
                    name: "a.lep".into()
                }
            );
            assert_eq!(edited.text(), "mine");
            assert!(edited.is_dirty() && backup.paths[0].exists());
            let compared = files.current_text("", "a.lep").unwrap().unwrap();
            std::fs::write(&path, "changed again").unwrap();
            assert!(matches!(
                controller
                    .overwrite(&edited, &Expected::Revision(compared.revision.unwrap()))
                    .unwrap(),
                SaveOutcome::Conflict { .. }
            ));
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "changed again");
            assert!(edited.is_dirty() && backup.paths[0].exists());
            let confirmed = files.current_text("", "a.lep").unwrap().unwrap();
            assert!(matches!(
                controller
                    .overwrite(&edited, &Expected::Revision(confirmed.revision.unwrap()))
                    .unwrap(),
                SaveOutcome::Saved { .. }
            ));
            assert_eq!(std::fs::read_to_string(path).unwrap(), "mine");
            assert!(!edited.is_dirty());
            assert!(!backup.paths[0].exists());
        });
    }

    #[test]
    fn untitled_document_can_use_its_own_tab_name_but_not_replace_an_existing_file() {
        crate::gtk_test::run(|| {
            let project = Project::new();
            let files = Files::Local(project.0.clone());
            let edited = doc(None, "escreva(1)\n");
            edited.buffer.set_modified(true);
            let docs = [edited.clone()];
            let controller = Documents::new(&files, "local", &docs);
            assert!(matches!(
                controller.save(&edited, Some(&edited.name())).unwrap(),
                SaveOutcome::Saved { .. }
            ));
            assert!(!edited.is_untitled());
            let other = doc(None, "escreva(2)\n");
            other.buffer.set_modified(true);
            let docs = [other.clone()];
            assert!(
                Documents::new(&files, "local", &docs)
                    .save(&other, Some(&other.name()))
                    .is_err()
            );
            assert!(other.is_untitled() && other.is_dirty());
            assert_eq!(
                std::fs::read_to_string(project.0.join(edited.name())).unwrap(),
                "escreva(1)\n"
            );
        });
    }

    #[test]
    fn save_as_and_reload_preserve_the_other_version_until_explicit_choice() {
        crate::gtk_test::run(|| {
            let project = Project::new();
            let files = Files::Local(project.0.clone());
            std::fs::write(project.0.join("a.lep"), "newer on disk").unwrap();
            let edited = doc(Some("a.lep"), "base");
            edited.buffer.set_text("mine");
            let docs = [edited.clone()];
            let controller = Documents::new(&files, "local", &docs);
            assert!(matches!(
                controller.save(&edited, None).unwrap(),
                SaveOutcome::Conflict { .. }
            ));
            assert!(matches!(
                controller.save_as(&edited, "copy.lep").unwrap(),
                SaveOutcome::Saved { .. }
            ));
            assert_eq!(
                std::fs::read_to_string(project.0.join("a.lep")).unwrap(),
                "newer on disk"
            );
            assert_eq!(
                std::fs::read_to_string(project.0.join("copy.lep")).unwrap(),
                "mine"
            );
            assert_eq!(edited.name(), "copy.lep");
            let other = doc(Some("a.lep"), "base");
            other.buffer.set_text("unsaved");
            controller.reload(&other).unwrap();
            assert_eq!(other.text(), "newer on disk");
            assert!(!other.is_dirty());
            other.buffer.set_text("after reload");
            assert!(matches!(
                controller.save(&other, None).unwrap(),
                SaveOutcome::Saved { .. }
            ));
        });
    }

    #[test]
    fn deleted_files_and_unversioned_documents_cannot_be_silently_upserted() {
        crate::gtk_test::run(|| {
            let project = Project::new();
            let files = Files::Local(project.0.clone());
            let edited = doc(Some("missing.lep"), "base");
            edited.buffer.set_text("mine");
            let controller = Documents::new(&files, "local", &[]);
            assert!(matches!(
                controller.save(&edited, None).unwrap(),
                SaveOutcome::Conflict { .. }
            ));
            assert!(!project.0.join("missing.lep").exists());
            // The file can reappear while the overwrite dialog is open.
            std::fs::write(project.0.join("missing.lep"), "reappeared").unwrap();
            assert!(matches!(
                controller.overwrite(&edited, &Expected::Missing).unwrap(),
                SaveOutcome::Conflict { .. }
            ));
            assert_eq!(
                std::fs::read_to_string(project.0.join("missing.lep")).unwrap(),
                "reappeared"
            );
            *edited.revision.borrow_mut() = None;
            assert!(controller.save(&edited, None).is_err());
            assert!(edited.is_dirty());
            assert_eq!(edited.text(), "mine");
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
                    warning: None
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
