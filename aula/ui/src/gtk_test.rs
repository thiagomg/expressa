//! Run GTK assertions on one thread. A display is required.
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Mutex, OnceLock};

type Job = (Box<dyn FnOnce() + Send>, Sender<std::thread::Result<()>>);

/// Runs `f` on the GTK thread. A display is required: never report an
/// unexecuted GTK assertion as a passing test.
pub fn run(f: impl FnOnce() + Send + 'static) {
    static JOBS: OnceLock<Option<Mutex<Sender<Job>>>> = OnceLock::new();
    let jobs = JOBS.get_or_init(|| {
        let (tx, rx) = channel::<Job>();
        let (ready_tx, ready_rx) = channel();
        std::thread::spawn(move || {
            let ok = gtk::init().is_ok();
            let _ = ready_tx.send(ok);
            if ok {
                for (job, done) in rx {
                    let _ = done.send(catch_unwind(AssertUnwindSafe(job)));
                }
            }
        });
        if ready_rx.recv().unwrap_or(false) {
            eprintln!("GTK initialized: executing GUI test assertions");
            Some(Mutex::new(tx))
        } else {
            None
        }
    });
    // Cache initialization failure too: retrying GTK on different threads
    // after a failure is unsafe and can crash the rest of the test process.
    let jobs = jobs.as_ref().expect(
        "GTK tests require a working display; run with a desktop display or a virtual display",
    );
    let (done_tx, done_rx) = channel();
    jobs.lock().unwrap().send((Box::new(f), done_tx)).unwrap();
    if let Err(panic) = done_rx.recv().unwrap() {
        resume_unwind(panic);
    }
}
