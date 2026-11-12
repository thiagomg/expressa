//! Benchmark-only allocator instrumentation. Counts requested bytes, not malloc
//! metadata or resident pages. Use identical instrumentation for both libraries.
use expressa::runtime::{ExecutionBackend, ExecutionOptions, run_with_options};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering::Relaxed};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static LIVE: AtomicIsize = AtomicIsize::new(0);
static PEAK: AtomicIsize = AtomicIsize::new(0);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static REALLOCS: AtomicUsize = AtomicUsize::new(0);
static FREES: AtomicUsize = AtomicUsize::new(0);
static REQUESTED: AtomicUsize = AtomicUsize::new(0);
struct Count;
#[global_allocator]
static ALLOCATOR: Count = Count;
fn update(delta: isize) {
    let live = LIVE.fetch_add(delta, Relaxed) + delta;
    if ACTIVE.load(Relaxed) {
        PEAK.fetch_max(live, Relaxed);
    }
}
unsafe impl GlobalAlloc for Count {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            update(layout.size() as isize);
            if ACTIVE.load(Relaxed) {
                ALLOCS.fetch_add(1, Relaxed);
                REQUESTED.fetch_add(layout.size(), Relaxed);
            }
        }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            update(layout.size() as isize);
            if ACTIVE.load(Relaxed) {
                ALLOCS.fetch_add(1, Relaxed);
                REQUESTED.fetch_add(layout.size(), Relaxed);
            }
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        update(-(layout.size() as isize));
        if ACTIVE.load(Relaxed) {
            FREES.fetch_add(1, Relaxed);
        }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let out = unsafe { System.realloc(ptr, layout, size) };
        if !out.is_null() {
            update(size as isize - layout.size() as isize);
            if ACTIVE.load(Relaxed) {
                REALLOCS.fetch_add(1, Relaxed);
                REQUESTED.fetch_add(size, Relaxed);
            }
        }
        out
    }
}
fn run(source: &str, file: &str, backend: ExecutionBackend) {
    let output = run_with_options(
        source,
        file,
        "",
        ExecutionOptions {
            backend,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(output.exit_code, 0);
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, "verdadeiro\n");
    drop(output);
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "usage: probe BACKEND PROGRAM");
    let backend = ExecutionBackend::from_name(&args[1]).unwrap();
    let source = std::fs::read_to_string(&args[2]).unwrap();
    run(&source, &args[2], backend); // Exclude first-time initialization.
    let baseline = LIVE.load(Relaxed);
    PEAK.store(baseline, Relaxed);
    ACTIVE.store(true, Relaxed);
    run(&source, &args[2], backend);
    ACTIVE.store(false, Relaxed);
    println!(
        "{{\"allocations\":{},\"reallocations\":{},\"deallocations\":{},\"requested_bytes\":{},\"peak_live_delta_bytes\":{},\"retained_delta_bytes\":{}}}",
        ALLOCS.load(Relaxed),
        REALLOCS.load(Relaxed),
        FREES.load(Relaxed),
        REQUESTED.load(Relaxed),
        PEAK.load(Relaxed) - baseline,
        LIVE.load(Relaxed) - baseline
    );
}
