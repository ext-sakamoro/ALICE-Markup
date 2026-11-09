#![no_main]
//! Any UTF-8 input parses without a panic, the same twice, with links only to
//! accepted targets and reader lines that are trimmed and single-spaced, and its text
//! layer has dense ids and style ranges that cover each block's text.

use alice_markup::{is_safe_href, parse, text_layer, Block, Inline};
use libfuzzer_sys::fuzz_target;

fn check(inlines: &[Inline], in_label: bool) {
    for inline in inlines {
        match inline {
            Inline::Link { label, href } => {
                assert!(!in_label, "a link inside a label");
                assert!(is_safe_href(href), "unsafe href {href:?}");
                check(label, true);
            }
            Inline::Emphasis(inner) | Inline::Strong(inner) => check(inner, in_label),
            Inline::Text(_) | Inline::Code(_) => {}
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(src) = core::str::from_utf8(data) else { return };
    let doc = parse(src);
    assert_eq!(doc, parse(src));
    for block in &doc.blocks {
        match block {
            Block::Heading { inlines, .. } | Block::Paragraph(inlines) | Block::Quote(inlines) => check(inlines, false),
            Block::List { items, .. } => items.iter().for_each(|i| check(i, false)),
            Block::Code { .. } => {}
        }
    }
    for line in doc.text_lines() {
        assert!(!line.is_empty() && line.trim() == line && !line.contains("  "));
    }
    // the text layer: dense ids, style ranges that cover each block's text, links on char boundaries
    let layer = text_layer(&doc);
    for (i, b) in layer.blocks.iter().enumerate() {
        assert_eq!(b.id as usize, i);
        let mut at = 0u32;
        for s in &b.spans {
            assert!(s.src.start == at && s.src.end > at);
            at = s.src.end;
        }
        assert_eq!(at as usize, b.text.len());
        for k in &b.links {
            assert!(k.src.start < k.src.end && b.text.is_char_boundary(k.src.start as usize) && b.text.is_char_boundary(k.src.end as usize));
        }
    }
});
