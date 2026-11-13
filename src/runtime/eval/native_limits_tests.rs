use super::*;
use std::cell::Cell;

struct StopAfter {
    polls: Rc<Cell<usize>>,
    stop_at: usize,
}
impl DebugHook for StopAfter {
    fn before_stmt(&mut self, _: &DebugCtx<'_>) -> DebugAction {
        DebugAction::Continue
    }
    fn interrupted(&mut self) -> bool {
        self.polls.set(self.polls.get() + 1);
        self.polls.get() >= self.stop_at
    }
}

fn stopped(source: &str, stop_at: usize, deadline: Option<Duration>) -> (i32, String, usize) {
    let polls = Rc::new(Cell::new(0));
    let mut input = io::Cursor::new("");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with(
        source,
        "stop.lep",
        Box::new(StopAfter {
            polls: polls.clone(),
            stop_at,
        }),
        &mut input,
        &mut out,
        &mut err,
        None,
        deadline,
        None,
        &[],
    )
    .unwrap();
    (code, String::from_utf8(out).unwrap(), polls.get())
}

#[test]
fn stop_is_polled_inside_matrix_multiplication() {
    // Construction uses fewer than 100 polls; stop occurs in multiplication.
    let (code, out, polls) = stopped(
        "importe \"matriz\"\nA = uns(60)\nB = A * A\nescreva(B)",
        200,
        None,
    );
    assert_eq!((code, out.as_str(), polls), (0, "", 200));
}

#[test]
fn stop_is_polled_inside_sorting() {
    let (code, out, polls) = stopped(
        "xs = []\npara i em 1..=100 { xs += [101 - i] }\nescreva(ordene(xs))",
        103,
        None,
    );
    assert_eq!((code, out.as_str(), polls), (0, "", 103));
}

#[test]
fn sleep_longer_than_deadline_can_be_stopped_immediately() {
    let start = Instant::now();
    let (code, out, polls) = stopped("durma(100)\nescreva(1)", 1, Some(Duration::from_secs(2)));
    assert_eq!((code, out.as_str(), polls), (0, "", 1));
    assert!(start.elapsed() < Duration::from_secs(1));
}
