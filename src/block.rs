//! Lines to blocks.

use alloc::string::ToString;
use alloc::vec::Vec;

use crate::inline::parse_inlines;
use crate::{Block, Document};

/// Parses `src` into a [`Document`]. Never fails, never panics; see the crate
/// documentation for the subset.
#[must_use]
pub fn parse(src: &str) -> Document {
    let normalised;
    let src = if src.contains('\r') {
        normalised = src.replace("\r\n", "\n").replace('\r', "\n");
        normalised.as_str()
    } else {
        src
    };
    let lines: Vec<&str> = src.split('\n').collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if is_blank(line) {
            i += 1;
            continue;
        }
        if let Some((ticks, info)) = fence_open(line) {
            let mut body: Vec<&str> = Vec::new();
            i += 1;
            while i < lines.len() && !fence_closes(lines[i], ticks) {
                body.push(lines[i]);
                i += 1;
            }
            i += 1; // the closing fence (or past the end)
            blocks.push(Block::Code {
                info: info.to_string(),
                text: body.join("\n"),
            });
            continue;
        }
        if let Some((level, text)) = heading(line) {
            blocks.push(Block::Heading {
                level,
                inlines: parse_inlines(text),
            });
            i += 1;
            continue;
        }
        if quote_body(line).is_some() {
            let mut parts: Vec<&str> = Vec::new();
            while i < lines.len() {
                let Some(body) = quote_body(lines[i]) else {
                    break;
                };
                parts.push(body.trim());
                i += 1;
            }
            blocks.push(Block::Quote(parse_inlines(&parts.join("\n"))));
            continue;
        }
        if let Some(first) = list_item(line) {
            let ordered = first.number.is_some();
            let start = first.number.unwrap_or(0);
            let mut items: Vec<Vec<crate::Inline>> = Vec::new();
            // lines[i] is an item of this list while the loop runs
            while let Some(item) = list_item(lines[i]) {
                let mut parts: Vec<&str> = alloc::vec![item.text.trim()];
                i += 1;
                while i < lines.len() && is_continuation(lines[i]) {
                    parts.push(lines[i].trim());
                    i += 1;
                }
                items.push(parse_inlines(&parts.join("\n")));
                // blank lines between two items of the same kind keep the list
                let mut j = i;
                while j < lines.len() && is_blank(lines[j]) {
                    j += 1;
                }
                match lines.get(j).and_then(|l| list_item(l)) {
                    Some(next) if next.number.is_some() == ordered => i = j,
                    _ => break,
                }
            }
            blocks.push(Block::List {
                ordered,
                start,
                items,
            });
            continue;
        }
        // a paragraph: up to a blank line or a line that starts another block
        let mut parts: Vec<&str> = alloc::vec![line.trim()];
        i += 1;
        while i < lines.len() && !is_blank(lines[i]) && !starts_block(lines[i]) {
            parts.push(lines[i].trim());
            i += 1;
        }
        blocks.push(Block::Paragraph(parse_inlines(&parts.join("\n"))));
    }
    Document { blocks }
}

fn is_blank(line: &str) -> bool {
    line.bytes().all(|b| b == b' ' || b == b'\t')
}

/// The line without up to three leading spaces, or `None` with four or more.
fn unindent(line: &str) -> Option<&str> {
    let spaces = line.bytes().take_while(|&b| b == b' ').count();
    (spaces <= 3).then(|| &line[spaces..])
}

fn starts_block(line: &str) -> bool {
    fence_open(line).is_some()
        || heading(line).is_some()
        || quote_body(line).is_some()
        || list_item(line).is_some()
}

/// `#`..`######` then a space or the end of the line: (level, text without a
/// closing hash run).
fn heading(line: &str) -> Option<(u8, &str)> {
    let rest = unindent(line)?;
    let hashes = rest.bytes().take_while(|&b| b == b'#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let after = &rest[hashes..];
    if !(after.is_empty() || after.starts_with(' ') || after.starts_with('\t')) {
        return None;
    }
    let mut text = after.trim();
    // a closing run of hashes, preceded by a space (or the whole text)
    let closing = text.bytes().rev().take_while(|&b| b == b'#').count();
    if closing > 0 {
        let before = &text[..text.len() - closing];
        if before.is_empty() || before.ends_with(' ') || before.ends_with('\t') {
            text = before.trim_end();
        }
    }
    u8::try_from(hashes).ok().map(|level| (level, text))
}

/// Three or more backticks, then an info string without backticks: (fence
/// length, first word of the info).
fn fence_open(line: &str) -> Option<(usize, &str)> {
    let rest = unindent(line)?;
    let ticks = rest.bytes().take_while(|&b| b == b'`').count();
    if ticks < 3 {
        return None;
    }
    let info = &rest[ticks..];
    if info.contains('`') {
        return None;
    }
    Some((ticks, info.split_whitespace().next().unwrap_or("")))
}

fn fence_closes(line: &str, ticks: usize) -> bool {
    let Some(rest) = unindent(line) else {
        return false;
    };
    let n = rest.bytes().take_while(|&b| b == b'`').count();
    n >= ticks && is_blank(&rest[n..])
}

/// The text after `>` and one optional space.
fn quote_body(line: &str) -> Option<&str> {
    let rest = unindent(line)?.strip_prefix('>')?;
    Some(rest.strip_prefix(' ').unwrap_or(rest))
}

struct Item<'a> {
    /// `Some(n)` for an ordered item.
    number: Option<u32>,
    text: &'a str,
}

/// `- ` / `* ` / `+ ` or 1 to 9 digits and `.` / `)`, then a space or the end
/// of the line.
fn list_item(line: &str) -> Option<Item<'_>> {
    let rest = unindent(line)?;
    let bytes = rest.as_bytes();
    let (number, marker_len) = match bytes.first()? {
        b'-' | b'*' | b'+' => (None, 1),
        b'0'..=b'9' => {
            let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
            if digits > 9 || !matches!(bytes.get(digits), Some(b'.' | b')')) {
                return None;
            }
            (rest[..digits].parse::<u32>().ok(), digits + 1)
        }
        _ => return None,
    };
    let after = &rest[marker_len..];
    if !(after.is_empty() || after.starts_with(' ') || after.starts_with('\t')) {
        return None;
    }
    Some(Item {
        number,
        text: after,
    })
}

/// An indented, non-blank line that starts no block of its own.
fn is_continuation(line: &str) -> bool {
    (line.starts_with(' ') || line.starts_with('\t')) && !is_blank(line) && !starts_block(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unindent_counts_spaces_only() {
        assert_eq!(unindent("   x"), Some("x"));
        assert_eq!(unindent("    x"), None);
        assert_eq!(unindent("\tx"), Some("\tx"));
    }

    #[test]
    fn list_item_number_fits_u32() {
        assert_eq!(
            list_item("999999999. x").and_then(|i| i.number),
            Some(999_999_999)
        );
    }
}
