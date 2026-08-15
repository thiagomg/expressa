//! Source locations for tokens, AST nodes, and diagnostics.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    /// 1-based line of the start of this span.
    pub line: u32,
    /// 1-based column of the start of this span (character-oriented).
    pub col: u32,
    /// Inclusive byte offset into the source.
    pub start: usize,
    /// Exclusive byte offset into the source.
    pub end: usize,
}

impl Span {
    /// Span from absolute fields.
    pub fn new(line: u32, column: u32, start: usize, end: usize) -> Self {
        debug_assert!(start <= end, "span start must be <= end");
        Self {
            line,
            col: column,
            start,
            end,
        }
    }

    /// Zero-width span at a cursor (e.g. EOF).
    pub fn empty(line: u32, column: u32, offset: usize) -> Self {
        Self {
            line,
            col: column,
            start: offset,
            end: offset,
        }
    }

    /// Span starting at `(line, column, start)` with length `len` bytes.
    pub fn at(line: u32, column: u32, start: usize, len: usize) -> Self {
        Self {
            line,
            col: column,
            start,
            end: start + len,
        }
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Length in bytes (`end - start`).
    pub fn len(self) -> usize {
        self.end - self.start
    }

    /// Substring of `source` covered by this span.
    ///
    /// # Panics
    /// Panics if the span is out of bounds or not on a UTF-8 boundary.
    pub fn lexeme<'a>(self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }

    /// Span from the start of `self` to the end of `other`.
    ///
    /// Assumes same file and left-to-right order. `line`/`column` come from `self`.
    pub fn join(self, other: Span) -> Span {
        Self {
            line: self.line,
            col: self.col,
            start: self.start,
            end: other.end.max(self.end),
        }
    }

    /// Span covering both ranges, order-independent.
    pub fn merge(self, other: Span) -> Span {
        if self.start <= other.start {
            Self {
                line: self.line,
                col: self.col,
                start: self.start,
                end: self.end.max(other.end),
            }
        } else {
            Self {
                line: other.line,
                col: other.col,
                start: other.start,
                end: self.end.max(other.end),
            }
        }
    }

    /// `"line:column"` of the start.
    pub fn location(self) -> String {
        format!("{}:{}", self.line, self.col)
    }

    /// `"file:line:column"` of the start.
    pub fn location_in(self, file: &str) -> String {
        format!("{}:{}:{}", file, self.line, self.col)
    }

    /// Copy with a new exclusive end offset.
    pub fn with_end(self, end: usize) -> Self {
        debug_assert!(self.start <= end, "with_end: end must be >= start");
        Self { end, ..self }
    }

    /// Zero-width span at the end of this span.
    pub fn end_point(self) -> Span {
        Self::empty(self.line, self.col, self.end)
    }

    /// Whether `offset` lies in `[start, end)`.
    pub fn contains_offset(self, offset: usize) -> bool {
        self.start <= offset && offset < self.end
    }

    /// Whether the byte ranges overlap.
    pub fn overlaps(self, other: Span) -> bool {
        self.start < other.end && other.start < self.end
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_zero_width() {
        let s = Span::empty(2, 5, 10);
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        assert_eq!(s.start, 10);
        assert_eq!(s.end, 10);
        assert_eq!(s.line, 2);
        assert_eq!(s.col, 5);
    }

    #[test]
    fn at_and_lexeme() {
        let src = "x = 10";
        let s = Span::at(1, 5, 4, 2);
        assert_eq!(s.lexeme(src), "10");
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn join_covers_left_to_right() {
        let left = Span::new(1, 1, 0, 1);
        let right = Span::new(1, 5, 4, 6);
        let j = left.join(right);
        assert_eq!(j.start, 0);
        assert_eq!(j.end, 6);
        assert_eq!(j.line, 1);
        assert_eq!(j.col, 1);
        assert_eq!(j.lexeme("x = 10"), "x = 10");
    }

    #[test]
    fn merge_order_independent() {
        let a = Span::new(1, 5, 4, 6);
        let b = Span::new(1, 1, 0, 1);
        assert_eq!(a.merge(b), b.merge(a));
        assert_eq!(a.merge(b).start, 0);
        assert_eq!(a.merge(b).end, 6);
    }

    #[test]
    fn location_and_display() {
        let s = Span::new(3, 12, 40, 45);
        assert_eq!(s.location(), "3:12");
        assert_eq!(s.location_in("main.lep"), "main.lep:3:12");
        assert_eq!(format!("{s}"), "3:12");
    }

    #[test]
    fn contains_and_overlaps() {
        let s = Span::new(1, 1, 2, 5);
        assert!(s.contains_offset(2));
        assert!(s.contains_offset(4));
        assert!(!s.contains_offset(5));
        assert!(!s.contains_offset(1));

        let t = Span::new(1, 1, 4, 8);
        assert!(s.overlaps(t));
        let u = Span::new(1, 1, 5, 8);
        assert!(!s.overlaps(u));
    }

    #[test]
    fn with_end_and_end_point() {
        let s = Span::new(1, 1, 0, 0).with_end(4);
        assert_eq!(s.end, 4);
        let p = s.end_point();
        assert!(p.is_empty());
        assert_eq!(p.start, 4);
    }
}
