//! A small, deterministic Markdown subset.
//!
//! [`parse`] turns text into a [`Document`]: a list of [`Block`]s (headings,
//! paragraphs, lists, quotes, fenced code) whose text is a list of [`Inline`]s
//! (plain text, emphasis, strong emphasis, code spans, links). An image is
//! read as its alt text, and an indented list item as one more item of its
//! list. Anything else (HTML, tables, setext headings, reference links, hard
//! breaks) is kept as plain text, so the reader still sees every character and
//! nothing is interpreted that the subset does not name.
//!
//! The same input always gives the same document, on every target: the parser
//! uses byte and `char` comparisons only (no floating point, no hashing, no
//! allocation-order effects). Every input is accepted: [`parse`] never fails
//! and never panics, and its running time is linear in the input length.
//!
//! [`Document::text_lines`] gives what a reader sees, one line per heading,
//! paragraph, quote, list item and code line, with whitespace runs collapsed:
//! the string a renderer must show, and the string a check compares when it
//! looks for a text in a built site.
//!
//! [`split_more`] cuts a text at its one `<!-- more -->` line into the part
//! everyone reads and the part kept back.
//!
//! ```
//! use alice_markup::{parse, split_more};
//!
//! let text = "# Notes\n\nOpen to everyone.\n<!-- more -->\nThe rest, with a [link](https://example.org).";
//! let (first, rest) = split_more(text).unwrap();
//! assert_eq!(parse(first).text_lines(), ["Notes", "Open to everyone."]);
//! assert_eq!(parse(rest).text_lines(), ["The rest, with a link."]);
//! ```

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod block;
mod inline;
mod more;
mod text;

pub use block::parse;
pub use more::{split_more, MoreError, MORE_MARKER};

use alloc::string::String;
use alloc::vec::Vec;

/// A parsed text: its blocks in reading order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Document {
    /// The blocks, in the order they appear.
    pub blocks: Vec<Block>,
}

/// One block of a [`Document`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// `#` to `######` followed by a space: a heading of that level (1 to 6).
    Heading {
        /// 1 to 6.
        level: u8,
        /// The heading text.
        inlines: Vec<Inline>,
    },
    /// Consecutive lines that start no other block; the line breaks inside are
    /// soft (read as one space).
    Paragraph(Vec<Inline>),
    /// Consecutive `- ` / `* ` items (unordered) or `1. ` items (ordered).
    List {
        /// Whether the items are numbered.
        ordered: bool,
        /// The number of the first item of an ordered list (0 for an unordered one).
        start: u32,
        /// One entry per item.
        items: Vec<Vec<Inline>>,
    },
    /// Consecutive lines starting with `>`.
    Quote(Vec<Inline>),
    /// A block between two lines of three or more backticks, taken verbatim.
    Code {
        /// The word after the opening backticks, if any.
        info: String,
        /// The lines between the fences, joined with `\n`.
        text: String,
    },
}

/// A run of text inside a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    /// Plain text, shown as it is (a renderer escapes it for its output).
    Text(String),
    /// `*text*` or `_text_`.
    Emphasis(Vec<Inline>),
    /// `**text**` or `__text__`.
    Strong(Vec<Inline>),
    /// `` `code` ``: shown verbatim.
    Code(String),
    /// `[label](href)` with an accepted `href` (see [`is_safe_href`]); a link
    /// whose target is not accepted is kept as its label, in plain text.
    Link {
        /// The label, which may hold emphasis and code but no link.
        label: Vec<Inline>,
        /// The target, as written.
        href: String,
    },
}

impl Document {
    /// What a reader sees, one entry per heading, paragraph, quote, list item
    /// and non-blank code line: the inline text joined (a link shows its label,
    /// emphasis its content), whitespace runs collapsed to one space, trimmed.
    /// Empty entries are left out.
    #[must_use]
    pub fn text_lines(&self) -> Vec<String> {
        text::lines(self)
    }
}

/// Whether `href` is a target a link may carry: `http://` / `https://` /
/// `mailto:` (letters compared without case), or a path or fragment on the
/// same site (`/…` but not `//…`, `#…`). Anything else (`javascript:`,
/// `data:`, a relative path, a scheme-relative `//host`) is refused.
#[must_use]
pub fn is_safe_href(href: &str) -> bool {
    if href.is_empty() || href.bytes().any(|b| b.is_ascii_control() || b == b' ') {
        return false;
    }
    let lower_starts = |p: &str| {
        href.len() >= p.len() && href.as_bytes()[..p.len()].eq_ignore_ascii_case(p.as_bytes())
    };
    if lower_starts("http://") || lower_starts("https://") {
        return href.len() > if lower_starts("https://") { 8 } else { 7 };
    }
    if lower_starts("mailto:") {
        return href.len() > 7;
    }
    (href.starts_with('/') && !href.starts_with("//")) || href.starts_with('#')
}
