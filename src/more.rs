//! The line that separates the part everyone reads from the part kept back.

/// The marker, alone on its line (surrounding spaces and tabs allowed).
pub const MORE_MARKER: &str = "<!-- more -->";

/// Why a text could not be split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoreError {
    /// No line is the marker.
    Missing,
    /// This many lines are the marker (two or more).
    Repeated(usize),
}

impl core::fmt::Display for MoreError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Missing => write!(f, "no line is {MORE_MARKER}"),
            Self::Repeated(n) => write!(f, "{n} lines are {MORE_MARKER}, expected exactly one"),
        }
    }
}

/// Splits `text` at its one marker line: the text before that line (with its
/// line break) and the text after it. A marker inside other text on a line
/// does not count. Lines end with `\n`; spaces, tabs and a `\r` around the
/// marker are ignored.
///
/// # Errors
///
/// [`MoreError::Missing`] without a marker line, [`MoreError::Repeated`] with
/// more than one.
pub fn split_more(text: &str) -> Result<(&str, &str), MoreError> {
    let mut found: Option<(usize, usize)> = None;
    let mut count = 0usize;
    let mut start = 0usize;
    for line in text.split_inclusive('\n') {
        let end = start + line.len();
        if line.trim_matches(|c| c == ' ' || c == '\t' || c == '\r' || c == '\n') == MORE_MARKER {
            count += 1;
            found.get_or_insert((start, end));
        }
        start = end;
    }
    match (count, found) {
        (1, Some((s, e))) => Ok((&text[..s], &text[e..])),
        (0, _) => Err(MoreError::Missing),
        (n, _) => Err(MoreError::Repeated(n)),
    }
}
