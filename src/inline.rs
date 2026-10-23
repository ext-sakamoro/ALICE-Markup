//! Text to inlines.

use alloc::string::String;
use alloc::vec::Vec;

use crate::{is_safe_href, Inline};

/// How far (in bytes) a closing delimiter, bracket or backtick run is looked
/// for. Past it the opener is plain text, which keeps parsing linear in the
/// input length.
pub(crate) const WINDOW: usize = 4096;

/// How deep emphasis and link labels nest; deeper delimiters are plain text.
pub(crate) const MAX_DEPTH: usize = 16;

/// Parses the text of one block (lines joined with `\n`, read as spaces).
pub(crate) fn parse_inlines(src: &str) -> Vec<Inline> {
    parse(src, true, 0)
}

struct Out {
    inlines: Vec<Inline>,
    text: String,
}

impl Out {
    fn add(&mut self, inline: Inline) {
        if let Inline::Text(t) = inline {
            self.text.push_str(&t);
        } else {
            self.flush();
            self.inlines.push(inline);
        }
    }

    fn flush(&mut self) {
        if !self.text.is_empty() {
            self.inlines
                .push(Inline::Text(core::mem::take(&mut self.text)));
        }
    }

    fn finish(mut self) -> Vec<Inline> {
        self.flush();
        self.inlines
    }
}

fn run_len(bytes: &[u8], at: usize, c: u8) -> usize {
    bytes[at..].iter().take_while(|&&b| b == c).count()
}

fn char_before(src: &str, at: usize) -> Option<char> {
    src[..at].chars().next_back()
}

fn char_at(src: &str, at: usize) -> Option<char> {
    src.get(at..).and_then(|s| s.chars().next())
}

/// The start of the closing run and the end of the code span whose opening run
/// of `ticks` backticks starts at `at`; `None` if it does not close within
/// [`WINDOW`].
fn code_span_end(bytes: &[u8], at: usize, ticks: usize) -> Option<(usize, usize)> {
    let limit = bytes.len().min(at + ticks + WINDOW);
    let mut pos = at + ticks;
    while pos < limit {
        if bytes[pos] == b'`' {
            let len = run_len(bytes, pos, b'`');
            if len == ticks {
                return Some((pos, pos + len));
            }
            pos += len;
        } else {
            pos += 1;
        }
    }
    None
}

/// `[label](href)` starting with the `[` at `at`: (label range, href, end).
/// The label holds no unescaped bracket; the href runs to the first `)` and
/// holds no whitespace.
fn link_at(src: &str, at: usize) -> Option<((usize, usize), &str, usize)> {
    let bytes = src.as_bytes();
    let limit = bytes.len().min(at + 1 + WINDOW);
    let mut pos = at + 1;
    let close = loop {
        if pos >= limit {
            return None;
        }
        match bytes[pos] {
            b'\\' => pos += 2,
            b'[' => return None,
            b']' => break pos,
            _ => pos += 1,
        }
    };
    if bytes.get(close + 1) != Some(&b'(') {
        return None;
    }
    let href_start = close + 2;
    let rel = bytes[href_start..bytes.len().min(href_start + WINDOW)]
        .iter()
        .position(|&b| b == b')')?;
    let href = &src[href_start..href_start + rel];
    if href.chars().any(char::is_whitespace) {
        return None;
    }
    Some(((at + 1, close), href, href_start + rel + 1))
}

/// The start of the closing run for an emphasis run of `run` bytes `delim`
/// starting at `at`, or `None`.
fn emphasis_close(src: &str, at: usize, delim: u8, run: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let limit = bytes.len().min(at + run + WINDOW);
    let mut pos = at + run;
    while pos < limit {
        match bytes[pos] {
            b'\\' => pos += 2,
            b'`' => {
                let ticks = run_len(bytes, pos, b'`');
                pos = code_span_end(bytes, pos, ticks).map_or(pos + ticks, |(_, end)| end);
            }
            b if b == delim => {
                let len = run_len(bytes, pos, delim);
                let after_ok =
                    delim == b'*' || !char_at(src, pos + len).is_some_and(char::is_alphanumeric);
                let before_ok =
                    pos > at + run && !char_before(src, pos).is_some_and(char::is_whitespace);
                if len == run && before_ok && after_ok {
                    return Some(pos);
                }
                pos += len;
            }
            _ => pos += 1,
        }
    }
    None
}

/// A backslash at `i`: an escaped ASCII punctuation character, or a backslash.
fn escape(out: &mut Out, bytes: &[u8], i: usize) -> usize {
    match bytes.get(i + 1) {
        Some(&b) if b.is_ascii_punctuation() => {
            out.text.push(char::from(b));
            i + 2
        }
        _ => {
            out.text.push('\\');
            i + 1
        }
    }
}

/// A backtick run at `i`: a code span, or the run as text.
fn code_span(out: &mut Out, src: &str, i: usize) -> usize {
    let bytes = src.as_bytes();
    let ticks = run_len(bytes, i, b'`');
    let Some((close, end)) = code_span_end(bytes, i, ticks) else {
        out.text.push_str(&src[i..i + ticks]);
        return i + ticks;
    };
    let mut code: String = src[i + ticks..close]
        .chars()
        .map(|ch| if ch == '\n' { ' ' } else { ch })
        .collect();
    if code.len() >= 2
        && code.starts_with(' ')
        && code.ends_with(' ')
        && !code.bytes().all(|b| b == b' ')
    {
        code = String::from(&code[1..code.len() - 1]);
    }
    out.add(Inline::Code(code));
    end
}

/// `[` at `i` (or the `!` of `![` when `image`): a link, an image's alt text,
/// the label of a link whose target is refused, or the character as text.
fn bracket(out: &mut Out, src: &str, i: usize, image: bool, depth: usize) -> usize {
    let open = if image { i + 1 } else { i };
    let Some(((s, e), href, end)) = link_at(src, open) else {
        out.text.push(if image { '!' } else { '[' });
        return i + 1;
    };
    let label = parse(&src[s..e], false, depth + 1);
    if !image && is_safe_href(href) {
        out.add(Inline::Link {
            label,
            href: String::from(href),
        });
    } else {
        // an image is read as its alt text; a refused link as its label
        for inline in label {
            out.add(inline);
        }
    }
    end
}

/// A run of `*` or `_` at `i`: emphasis or strong emphasis, or the run as text.
fn emphasis(out: &mut Out, src: &str, i: usize, links: bool, depth: usize) -> usize {
    let bytes = src.as_bytes();
    let delim = bytes[i];
    let run = run_len(bytes, i, delim);
    let opens = run <= 2
        && depth < MAX_DEPTH
        && char_at(src, i + run).is_some_and(|ch| !ch.is_whitespace())
        && (delim == b'*' || !char_before(src, i).is_some_and(char::is_alphanumeric));
    if let Some(close) = opens.then(|| emphasis_close(src, i, delim, run)).flatten() {
        let inner = parse(&src[i + run..close], links, depth + 1);
        out.add(if run == 1 {
            Inline::Emphasis(inner)
        } else {
            Inline::Strong(inner)
        });
        close + run
    } else {
        out.text.push_str(&src[i..i + run]);
        i + run
    }
}

fn parse(src: &str, links: bool, depth: usize) -> Vec<Inline> {
    let bytes = src.as_bytes();
    let mut out = Out {
        inlines: Vec::new(),
        text: String::new(),
    };
    let nest = links && depth < MAX_DEPTH;
    let mut i = 0;
    while i < bytes.len() {
        i = match bytes[i] {
            b'\n' => {
                out.text.push(' ');
                i + 1
            }
            b'\\' => escape(&mut out, bytes, i),
            b'`' => code_span(&mut out, src, i),
            b'!' if nest && bytes.get(i + 1) == Some(&b'[') => {
                bracket(&mut out, src, i, true, depth)
            }
            b'[' if nest => bracket(&mut out, src, i, false, depth),
            b'*' | b'_' => emphasis(&mut out, src, i, links, depth),
            _ => {
                let ch = char_at(src, i).unwrap_or('\u{fffd}');
                out.text.push(ch);
                i + ch.len_utf8()
            }
        };
    }
    out.finish()
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{Inline, MAX_DEPTH};
    use alloc::string::ToString;
    use alloc::vec;
    use alloc::vec::Vec;

    fn t(s: &str) -> Inline {
        Inline::Text(s.to_string())
    }

    /// The recursion under a deadline (see `alice_markup_test_bounds::within`).
    fn parse(src: &str, links: bool, depth: usize) -> Vec<Inline> {
        let src = src.to_string();
        alice_markup_test_bounds::within(std::time::Duration::from_secs(10), move || {
            super::parse(&src, links, depth)
        })
    }

    fn link(label: Vec<Inline>) -> Inline {
        Inline::Link {
            label,
            href: "/x".to_string(),
        }
    }

    // The emphasis kinds (`*`, `_`, `**`, `__`) and labels without `[` keep
    // public inputs far below MAX_DEPTH, so the depth limit is pinned here, at
    // the recursion itself, one level each side of it.

    #[test]
    fn emphasis_opens_below_max_depth_only() {
        let depth = MAX_DEPTH - 1;
        assert_eq!(parse("*a*", true, depth), [Inline::Emphasis(vec![t("a")])]);
        assert_eq!(parse("*a*", true, MAX_DEPTH), [t("*a*")]);
        assert_eq!(parse("*a*", true, MAX_DEPTH + 1), [t("*a*")]);
    }

    #[test]
    fn the_content_of_emphasis_is_one_level_deeper() {
        assert_eq!(
            parse("*_a_*", true, MAX_DEPTH - 1),
            [Inline::Emphasis(vec![t("_a_")])]
        );
        assert_eq!(
            parse("*_a_*", true, MAX_DEPTH - 2),
            [Inline::Emphasis(vec![Inline::Emphasis(vec![t("a")])])]
        );
    }

    #[test]
    fn links_open_below_max_depth_only() {
        assert_eq!(parse("[a](/x)", true, MAX_DEPTH - 1), [link(vec![t("a")])]);
        assert_eq!(parse("[a](/x)", true, MAX_DEPTH), [t("[a](/x)")]);
        assert_eq!(parse("![a](/x)", true, MAX_DEPTH), [t("![a](/x)")]);
        assert_eq!(parse("![a](/x)", true, MAX_DEPTH - 1), [t("a")]);
    }

    #[test]
    fn a_label_holds_no_link_and_is_one_level_deeper() {
        // inside a label (`links` false) brackets are text at any depth
        assert_eq!(parse("[a](/x)", false, 0), [t("[a](/x)")]);
        assert_eq!(parse("![a](/x)", false, 0), [t("![a](/x)")]);
        assert_eq!(
            parse("[*a*](/x)", true, MAX_DEPTH - 1),
            [link(vec![t("*a*")])]
        );
        assert_eq!(
            parse("[*a*](/x)", true, MAX_DEPTH - 2),
            [link(vec![Inline::Emphasis(vec![t("a")])])]
        );
    }
}
