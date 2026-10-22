//! What a reader sees.

use alloc::string::String;
use alloc::vec::Vec;

use crate::{Block, Document, Inline};

fn append_text(out: &mut String, inlines: &[Inline]) {
    for inline in inlines {
        match inline {
            Inline::Text(t) | Inline::Code(t) => out.push_str(t),
            Inline::Emphasis(inner) | Inline::Strong(inner) => append_text(out, inner),
            Inline::Link { label, .. } => append_text(out, label),
        }
    }
}

/// Whitespace runs to one space, trimmed.
fn collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for word in s.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

fn inline_line(inlines: &[Inline]) -> String {
    let mut raw = String::new();
    append_text(&mut raw, inlines);
    collapse(&raw)
}

pub(crate) fn lines(doc: &Document) -> Vec<String> {
    let mut out = Vec::new();
    for block in &doc.blocks {
        match block {
            Block::Heading { inlines, .. } | Block::Paragraph(inlines) | Block::Quote(inlines) => {
                out.push(inline_line(inlines));
            }
            Block::List { items, .. } => out.extend(items.iter().map(|item| inline_line(item))),
            Block::Code { text, .. } => out.extend(text.split('\n').map(collapse)),
        }
    }
    out.retain(|line| !line.is_empty());
    out
}
