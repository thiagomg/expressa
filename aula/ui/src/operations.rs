//! Per-operation progress and queued-work cancellation, without disabling editing.
use gtk::{Box as GtkBox, Button, Label, MenuButton, Orientation, Popover, prelude::*};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub struct Operations {
    pub button: MenuButton,
    rows: GtkBox,
    next: Cell<u64>,
    active: RefCell<Vec<(u64, GtkBox, Arc<AtomicBool>, bool)>>,
}
pub struct Operation {
    owner: Weak<Operations>,
    id: u64,
}
impl Operations {
    pub fn new() -> Rc<Self> {
        let rows = GtkBox::new(Orientation::Vertical, 6);
        let popover = Popover::builder().child(&rows).build();
        let button = MenuButton::builder()
            .label("Operações")
            .popover(&popover)
            .visible(false)
            .build();
        Rc::new(Self {
            button,
            rows,
            next: Cell::new(0),
            active: RefCell::new(Vec::new()),
        })
    }
    pub fn begin(
        self: &Rc<Self>,
        name: &str,
        flag: Arc<AtomicBool>,
        cancellable: bool,
    ) -> Operation {
        let id = self.next.get() + 1;
        self.next.set(id);
        let row = GtkBox::new(Orientation::Horizontal, 8);
        row.append(&Label::new(Some(name)));
        if cancellable {
            let cancel = Button::with_label("Cancelar");
            let flag = flag.clone();
            cancel.connect_clicked(move |button| {
                flag.store(true, Ordering::Release);
                button.set_sensitive(false);
            });
            row.append(&cancel);
        }
        self.rows.append(&row);
        self.active.borrow_mut().push((id, row, flag, cancellable));
        self.button.set_visible(true);
        Operation {
            owner: Rc::downgrade(self),
            id,
        }
    }
    pub fn cancel_all(&self) {
        for (_, _, flag, cancellable) in self.active.borrow().iter() {
            if *cancellable {
                flag.store(true, Ordering::Release);
            }
        }
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.upgrade() {
            let mut active = owner.active.borrow_mut();
            if let Some(i) = active.iter().position(|(id, _, _, _)| *id == self.id) {
                let (_, row, _, _) = active.remove(i);
                owner.rows.remove(&row);
            }
            owner.button.set_visible(!active.is_empty());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_targets_reads_and_preserves_non_cancellable_writes() {
        crate::gtk_test::run(|| {
            let operations = Operations::new();
            let read = Arc::new(AtomicBool::new(false));
            let write = Arc::new(AtomicBool::new(false));
            let reading = operations.begin("Abrindo arquivo", read.clone(), true);
            let writing = operations.begin("Gravando cópia", write.clone(), false);
            operations.cancel_all();
            assert!(read.load(Ordering::Acquire));
            assert!(!write.load(Ordering::Acquire));
            drop(reading);
            assert!(operations.button.is_visible());
            drop(writing);
            assert!(!operations.button.is_visible());
        });
    }
}
