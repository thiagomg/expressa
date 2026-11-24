//! Any text must parse or fail with a `ParseError`, never panic or
//! overflow the stack.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|src: &str| {
    let _ = expressa::parser::parse(src);
});
