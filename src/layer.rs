//! The text layer: one document as HTML (for screen readers and search) and,
//! per block, the plain text a page lays out and draws.

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::ops::Range;

use crate::{Block, Document, Inline};

/// How a run of text is set: the inline marks around it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct InlineStyle {
    /// Inside `*…*` or `_…_`.
    pub emphasis: bool,
    /// Inside `**…**` or `__…__`.
    pub strong: bool,
    /// A code span, or a fenced code block.
    pub code: bool,
}

impl InlineStyle {
    /// What two styles share: a space between two runs takes this.
    #[must_use]
    pub const fn common(self, other: Self) -> Self {
        Self {
            emphasis: self.emphasis && other.emphasis,
            strong: self.strong && other.strong,
            code: self.code && other.code,
        }
    }
}

/// What a block is, for the page and for the HTML element it becomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    /// A heading of this level (1 to 6).
    Heading(u8),
    /// A paragraph.
    Paragraph,
    /// One item of a list; `number` is the item's number in an ordered list
    /// (the list's start plus the item's index) and 0 in an unordered one.
    ListItem {
        /// Whether the list is numbered.
        ordered: bool,
        /// The item's number, 0 when not numbered.
        number: u32,
    },
    /// A quote.
    Quote,
    /// A fenced code block.
    Code,
}

/// A range of a block's text set in one style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleRange {
    /// Byte range in [`BlockRun::text`].
    pub src: Range<u32>,
    /// The style of that range.
    pub style: InlineStyle,
}

/// A range of a block's text that is one link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkSpan {
    /// Byte range in [`BlockRun::text`]: the link's label as it is shown.
    pub src: Range<u32>,
    /// The link's target, as written (already accepted by [`crate::is_safe_href`]).
    pub href: String,
}

/// One block of the layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockRun {
    /// The block's number in reading order, from 0 with no gaps; the HTML
    /// element of the block has the id `b<id>`.
    pub id: u32,
    /// What the block is.
    pub role: Role,
    /// What the reader sees: the inline text with whitespace runs collapsed to
    /// one space and trimmed (as [`Document::text_lines`]); a code block keeps
    /// its lines and spaces as written.
    pub text: String,
    /// The styles over `text`: neighbouring ranges, no gap, no overlap, no two
    /// neighbours with one style, together exactly `0..text.len()`.
    pub spans: Vec<StyleRange>,
    /// The links in `text`, in order.
    pub links: Vec<LinkSpan>,
}

/// One document as a text layer: [`TextLayer::html`] and [`TextLayer::blocks`]
/// say the same text, block by block.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextLayer {
    /// The blocks as an HTML fragment: `<h1>`…`<h6>`, `<p>`, `<ul>` / `<ol>`
    /// with `<li>`, `<blockquote>`, `<pre><code>`, and inside them `<em>`,
    /// `<strong>`, `<code>`, `<a href>`; each block element has the id
    /// `b<id>`; `& < > " '` are escaped. One element per line.
    pub html: String,
    /// The blocks with visible text, in reading order. A block whose text is
    /// empty is left out (as in [`Document::text_lines`]) and has no element.
    pub blocks: Vec<BlockRun>,
}

/// The text layer of `doc`.
///
/// A space between two runs of different style takes the style both share,
/// and belongs to a link only when the characters on both sides are in that
/// same link; so a link's range is exactly its label, and the spaces around a
/// link are outside it.
#[must_use]
pub fn text_layer(doc: &Document) -> TextLayer {
    let mut out = TextLayer::default();
    let mut links: Vec<String> = Vec::new();
    for block in &doc.blocks {
        match block {
            Block::Heading { level, inlines } => {
                push_inline_block(
                    &mut out,
                    Role::Heading(*level),
                    inlines,
                    &mut links,
                    &["h", &level.to_string()].concat(),
                    "",
                    "",
                );
            }
            Block::Paragraph(inlines) => {
                push_inline_block(&mut out, Role::Paragraph, inlines, &mut links, "p", "", "");
            }
            Block::Quote(inlines) => {
                push_inline_block(
                    &mut out,
                    Role::Quote,
                    inlines,
                    &mut links,
                    "blockquote",
                    "<p>",
                    "</p>",
                );
            }
            Block::List {
                ordered,
                start,
                items,
            } => push_list(&mut out, *ordered, *start, items, &mut links),
            Block::Code { text, .. } => push_code(&mut out, text),
        }
    }
    out
}

/// One visible character with its style and the link it is in (an index
/// into the link targets, so two neighbouring links with one target stay two).
#[derive(Clone, Copy)]
struct Ch {
    c: char,
    style: InlineStyle,
    link: Option<usize>,
}

fn walk(
    inlines: &[Inline],
    style: InlineStyle,
    link: Option<usize>,
    links: &mut Vec<String>,
    raw: &mut Vec<Ch>,
) {
    for inline in inlines {
        match inline {
            Inline::Text(t) => raw.extend(t.chars().map(|c| Ch { c, style, link })),
            Inline::Code(t) => {
                let style = InlineStyle {
                    code: true,
                    ..style
                };
                raw.extend(t.chars().map(|c| Ch { c, style, link }));
            }
            Inline::Emphasis(inner) => walk(
                inner,
                InlineStyle {
                    emphasis: true,
                    ..style
                },
                link,
                links,
                raw,
            ),
            Inline::Strong(inner) => walk(
                inner,
                InlineStyle {
                    strong: true,
                    ..style
                },
                link,
                links,
                raw,
            ),
            Inline::Link { label, href } => {
                links.push(href.clone());
                let id = links.len() - 1;
                walk(label, style, Some(id), links, raw);
            }
        }
    }
}

/// The visible characters of one block: whitespace runs collapsed to one space
/// that takes what both neighbours share, leading and trailing whitespace dropped.
fn flatten(inlines: &[Inline], links: &mut Vec<String>) -> Vec<Ch> {
    let mut raw = Vec::new();
    walk(inlines, InlineStyle::default(), None, links, &mut raw);
    let mut out: Vec<Ch> = Vec::with_capacity(raw.len());
    let mut pending = false;
    for ch in raw {
        if ch.c.is_whitespace() {
            pending = true;
            continue;
        }
        if pending {
            if let Some(prev) = out.last().copied() {
                let link = if prev.link == ch.link { ch.link } else { None };
                out.push(Ch {
                    c: ' ',
                    style: prev.style.common(ch.style),
                    link,
                });
            }
            pending = false;
        }
        out.push(ch);
    }
    out
}

fn next_id(out: &TextLayer) -> u32 {
    u32::try_from(out.blocks.len()).unwrap_or(u32::MAX)
}

fn len_u32(s: &str) -> u32 {
    u32::try_from(s.len()).unwrap_or(u32::MAX)
}

fn push_inline_block(
    out: &mut TextLayer,
    role: Role,
    inlines: &[Inline],
    links: &mut Vec<String>,
    tag: &str,
    inner_open: &str,
    inner_close: &str,
) {
    let chars = flatten(inlines, links);
    if !chars.is_empty() {
        push_chars(out, role, &chars, links, tag, "", inner_open, inner_close);
    }
}

/// `attrs` follow the id in the opening tag (empty, or starting with a space)
#[allow(clippy::too_many_arguments)]
fn push_chars(
    out: &mut TextLayer,
    role: Role,
    chars: &[Ch],
    links: &[String],
    tag: &str,
    attrs: &str,
    inner_open: &str,
    inner_close: &str,
) {
    let id = next_id(out);
    let mut text = String::new();
    let mut spans: Vec<StyleRange> = Vec::new();
    let mut link_spans: Vec<LinkSpan> = Vec::new();
    let mut open_link: Option<(usize, u32)> = None;
    push_all(
        &mut out.html,
        &[
            "<",
            tag,
            " id=\"b",
            &id.to_string(),
            "\"",
            attrs,
            ">",
            inner_open,
        ],
    );
    let mut i = 0;
    while i < chars.len() {
        // one segment: the longest run of one style and one link
        let (style, link) = (chars[i].style, chars[i].link);
        let mut j = i;
        let mut seg = String::new();
        while j < chars.len() && chars[j].style == style && chars[j].link == link {
            seg.push(chars[j].c);
            j += 1;
        }
        // links open and close around segments
        if open_link.map(|(l, _)| Some(l)) != Some(link) {
            if let Some((l, start)) = open_link.take() {
                out.html.push_str("</a>");
                link_spans.push(LinkSpan {
                    src: start..len_u32(&text),
                    href: links[l].clone(),
                });
            }
            if let Some(l) = link {
                out.html.push_str("<a href=\"");
                escape_into(&mut out.html, &links[l]);
                out.html.push_str("\">");
                open_link = Some((l, len_u32(&text)));
            }
        }
        let start = len_u32(&text);
        text.push_str(&seg);
        let end = len_u32(&text);
        match spans.last_mut() {
            Some(last) if last.style == style => last.src.end = end,
            _ => spans.push(StyleRange {
                src: start..end,
                style,
            }),
        }
        let tags: &[(bool, &str)] = &[
            (style.strong, "strong"),
            (style.emphasis, "em"),
            (style.code, "code"),
        ];
        for (on, t) in tags {
            if *on {
                push_all(&mut out.html, &["<", t, ">"]);
            }
        }
        escape_into(&mut out.html, &seg);
        for (on, t) in tags.iter().rev() {
            if *on {
                push_all(&mut out.html, &["</", t, ">"]);
            }
        }
        i = j;
    }
    if let Some((l, start)) = open_link.take() {
        out.html.push_str("</a>");
        link_spans.push(LinkSpan {
            src: start..len_u32(&text),
            href: links[l].clone(),
        });
    }
    push_all(&mut out.html, &[inner_close, "</", tag, ">\n"]);
    out.blocks.push(BlockRun {
        id,
        role,
        text,
        spans,
        links: link_spans,
    });
}

fn escape_into(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
}

fn push_list(
    out: &mut TextLayer,
    ordered: bool,
    start: u32,
    items: &[Vec<Inline>],
    links: &mut Vec<String>,
) {
    let mut opened = false;
    for (i, item) in items.iter().enumerate() {
        let number = if ordered {
            start.saturating_add(u32::try_from(i).unwrap_or(u32::MAX))
        } else {
            0
        };
        let chars = flatten(item, links);
        if chars.is_empty() {
            continue;
        }
        if !opened {
            if ordered {
                // the list starts at the first item shown, so the HTML numbers what the blocks number
                push_all(
                    &mut out.html,
                    &["<ol start=\"", &number.to_string(), "\">\n"],
                );
            } else {
                out.html.push_str("<ul>\n");
            }
            opened = true;
        }
        // each item of a numbered list carries its own number
        let value = if ordered {
            [" value=\"", &number.to_string(), "\""].concat()
        } else {
            String::new()
        };
        push_chars(
            out,
            Role::ListItem { ordered, number },
            &chars,
            links,
            "li",
            &value,
            "",
            "",
        );
    }
    if opened {
        out.html
            .push_str(if ordered { "</ol>\n" } else { "</ul>\n" });
    }
}

fn push_code(out: &mut TextLayer, text: &str) {
    if text.split('\n').all(|line| line.trim().is_empty()) {
        return;
    }
    let id = next_id(out);
    let len = len_u32(text);
    push_all(
        &mut out.html,
        &["<pre id=\"b", &id.to_string(), "\"><code>"],
    );
    escape_into(&mut out.html, text);
    out.html.push_str("</code></pre>\n");
    out.blocks.push(BlockRun {
        id,
        role: Role::Code,
        text: text.into(),
        spans: alloc::vec![StyleRange {
            src: 0..len,
            style: InlineStyle {
                code: true,
                ..InlineStyle::default()
            }
        }],
        links: Vec::new(),
    });
}

fn push_all(out: &mut String, parts: &[&str]) {
    for part in parts {
        out.push_str(part);
    }
}
