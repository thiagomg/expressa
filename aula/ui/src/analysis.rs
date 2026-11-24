//! Immutable analysis of one buffer revision. Built on the analysis workers;
//! cursor movement only looks up cached character ranges.
use crate::docs::{Diag, diagnose};
use std::collections::{BTreeSet, HashMap};

type Range = (i32, i32);
type Pair = (Range, Range);
struct Token {
    start: i32,
    end: i32,
    pair: Pair,
}
#[derive(Default)]
struct Line {
    tokens: Vec<Token>,
    leading: Option<Pair>,
    spanning: Option<Pair>,
}

pub struct Analysis {
    pub lines: Option<BTreeSet<u32>>,
    pub diag: Option<Diag>,
    blocks: HashMap<i32, Line>,
    names: Vec<(i32, i32, String)>,
    occurrences: HashMap<String, Vec<Range>>,
}
impl Analysis {
    pub fn new(text: &str) -> Self {
        let lines = crate::debug_sources::executable_lines(text);
        let diag = if lines.is_none() {
            diagnose(text)
        } else {
            None
        };
        let chars: Vec<usize> = text.char_indices().map(|(b, _)| b).collect();
        let newlines: Vec<usize> = text.match_indices('\n').map(|(b, _)| b).collect();
        let char_at = |b: usize| chars.partition_point(|&p| p < b) as i32;
        let line_at = |b: usize| newlines.partition_point(|&p| p < b) as i32;
        let tokens = crate::indent::block_tokens(text);
        let mut mates = vec![None; tokens.len()];
        let mut stack = Vec::new();
        for (i, (open, _, _)) in tokens.iter().enumerate() {
            if *open {
                stack.push(i);
            } else if let Some(other) = stack.pop() {
                mates[i] = Some(other);
                mates[other] = Some(i);
            }
        }
        let mut blocks: HashMap<i32, Line> = HashMap::new();
        for (i, &(open, start, end)) in tokens.iter().enumerate() {
            let Some(other) = mates[i] else {
                continue;
            };
            let (_, a, b) = tokens[other];
            let pair = if open {
                ((char_at(start), char_at(end)), (char_at(a), char_at(b)))
            } else {
                ((char_at(a), char_at(b)), (char_at(start), char_at(end)))
            };
            let row = line_at(start);
            let line = blocks.entry(row).or_default();
            if !open && line.tokens.is_empty() {
                let start_line = if row == 0 {
                    0
                } else {
                    newlines[row as usize - 1] + 1
                };
                if text[start_line..start].trim().is_empty() {
                    line.leading = Some(pair);
                }
            }
            if open && line_at(a) > row {
                line.spanning = Some(pair);
            }
            line.tokens.push(Token {
                start: char_at(start),
                end: char_at(end),
                pair,
            });
        }
        let mut names = Vec::new();
        let mut occurrences: HashMap<String, Vec<Range>> = HashMap::new();
        // Keep the previous occurrence-highlighting size policy, but scan once.
        if text.len() <= 200_000 {
            let mut start = None;
            for (offset, c) in text.chars().chain(std::iter::once(' ')).enumerate() {
                if c.is_alphanumeric() || c == '_' {
                    start.get_or_insert(offset);
                } else if let Some(start) = start.take() {
                    let name: String =
                        text[chars[start]..chars.get(offset).copied().unwrap_or(text.len())].into();
                    if offset - start >= 2
                        && !name.chars().next().unwrap().is_numeric()
                        && !crate::lang::KEYWORDS.contains(&name.as_str())
                    {
                        occurrences
                            .entry(name.clone())
                            .or_default()
                            .push((start as i32, offset as i32));
                        names.push((start as i32, offset as i32, name));
                    }
                }
            }
        }
        Self {
            lines,
            diag,
            blocks,
            names,
            occurrences,
        }
    }
    pub fn block(&self, row: i32, cursor: i32) -> Option<Pair> {
        let line = self.blocks.get(&row)?;
        let i = line.tokens.partition_point(|t| t.end < cursor);
        line.tokens
            .get(i)
            .filter(|t| t.start <= cursor && cursor <= t.end)
            .map(|t| t.pair)
            .or(line.leading)
            .or(line.spanning)
    }
    pub fn occurrences(&self, cursor: i32) -> &[Range] {
        let i = self.names.partition_point(|(_, end, _)| *end < cursor);
        let Some((start, _, name)) = self.names.get(i).filter(|(start, _, _)| *start <= cursor)
        else {
            return &[];
        };
        let _ = start;
        self.occurrences
            .get(name)
            .filter(|ranges| ranges.len() > 1)
            .map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_blocks_and_occurrences_preserve_editor_behavior_at_every_cursor_position() {
        for text in [
            "se ação inicio\n nome = nome + 1\n fim\n",
            "se verdadeiro {\n se x { x = 1 }\n}\n",
            "// { inicio\nescreva(\"fim }\")\nse a {\n x = 2\n}\n",
            "posição = 1\nescreva(posição)\nx = x\nleia(leia)\n123abc 123abc\n",
            "se a {\n",
            "",
            "a {}{}\n",
            "se a {\n} senao {\n}\n",
        ] {
            let analysis = Analysis::new(text);
            for (cursor, byte) in text
                .char_indices()
                .map(|(b, _)| b)
                .chain(std::iter::once(text.len()))
                .enumerate()
            {
                let expected = crate::indent::block_pair(text, byte).map(|(a, b)| {
                    let chars = |(a, b)| {
                        (
                            text[..a].chars().count() as i32,
                            text[..b].chars().count() as i32,
                        )
                    };
                    (chars(a), chars(b))
                });
                assert_eq!(
                    analysis.block(text[..byte].matches('\n').count() as i32, cursor as i32),
                    expected,
                    "cursor {cursor}: {text}"
                );
                let expected: Vec<_> = crate::docs::ocorrencias_da_palavra(text, cursor)
                    .into_iter()
                    .map(|(a, b)| (a as i32, b as i32))
                    .collect();
                assert_eq!(
                    analysis.occurrences(cursor as i32),
                    expected,
                    "cursor {cursor}: {text}"
                );
            }
        }
    }

    #[test]
    #[ignore = "measurement scenario; run explicitly with --ignored --nocapture"]
    fn responsiveness_measurements_for_lesson_and_large_documents() {
        crate::gtk_test::run(|| {
            use gtk::prelude::*;
            use std::time::{Duration, Instant};
            for lines in [500, 5_000, 50_000] {
                let text = "ação = 1\nescreva(ação)\n".repeat(lines / 2);
                let start = Instant::now();
                let doc = crate::docs::Doc::new(
                    None,
                    "benchmark.lep".into(),
                    &text,
                    &sourceview5::LanguageManager::new(),
                    &sourceview5::SearchSettings::new(),
                );
                let created = start.elapsed();
                let ticks = std::rc::Rc::new(std::cell::Cell::new(0));
                let count = ticks.clone();
                let heartbeat = gtk::glib::timeout_add_local(Duration::from_millis(1), move || {
                    count.set(count.get() + 1);
                    gtk::glib::ControlFlow::Continue
                });
                let edit = Instant::now();
                doc.buffer.insert(
                    &mut doc.buffer.start_iter(),
                    "// typed while analysis is pending\n",
                );
                let typing = edit.elapsed();
                doc.check_now();
                let analysis = Instant::now();
                doc.wait_for_analysis();
                eprintln!(
                    "lines={lines} bytes={} buffer_creation={created:?} typing={typing:?} analysis_wall={:?} main_loop_ticks={}",
                    text.len(),
                    analysis.elapsed(),
                    ticks.get()
                );
                assert!(ticks.get() > 0);
                heartbeat.remove();
            }
        });
    }
}
