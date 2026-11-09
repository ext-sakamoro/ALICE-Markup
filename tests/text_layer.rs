//! The text layer: from one document, the HTML for screen readers and search
//! and, per block, the plain text a page lays out and draws, with its style
//! spans and link spans.
//!
//! oracle: each expected value below is written by hand from the rules in the
//! documentation of `text_layer` (block ids in reading order, whitespace
//! collapsed as `Document::text_lines` does, a space between two runs takes
//! what both runs share, HTML escaping of `& < > " '`), never from the output.
//! The property tests compare the layer with `Document::text_lines`, which is
//! written independently of it.

/// The same memory ceiling as the other test files (see `tests/oracle.rs`).
#[global_allocator]
static BUDGET: alice_markup_test_bounds::Budget = alice_markup_test_bounds::Budget::new(64 << 20);

use alice_markup::{parse, text_layer, BlockRun, InlineStyle, LinkSpan, Role, StyleRange};

fn layer(src: &str) -> alice_markup::TextLayer {
    let src = src.to_owned();
    alice_markup_test_bounds::within(std::time::Duration::from_secs(10), move || {
        text_layer(&parse(&src))
    })
}

const PLAIN: InlineStyle = InlineStyle {
    emphasis: false,
    strong: false,
    code: false,
};
const EM: InlineStyle = InlineStyle {
    emphasis: true,
    strong: false,
    code: false,
};
const STRONG: InlineStyle = InlineStyle {
    emphasis: false,
    strong: true,
    code: false,
};
const CODE: InlineStyle = InlineStyle {
    emphasis: false,
    strong: false,
    code: true,
};

fn span(a: u32, b: u32, style: InlineStyle) -> StyleRange {
    StyleRange { src: a..b, style }
}

#[test]
fn a_heading_and_a_paragraph_with_emphasis() {
    let l = layer("# Title\n\nHello *big*   world\n");
    assert_eq!(
        l.blocks,
        vec![
            BlockRun {
                id: 0,
                role: Role::Heading(1),
                text: "Title".into(),
                spans: vec![span(0, 5, PLAIN)],
                links: vec![]
            },
            BlockRun {
                id: 1,
                role: Role::Paragraph,
                text: "Hello big world".into(),
                spans: vec![span(0, 6, PLAIN), span(6, 9, EM), span(9, 15, PLAIN)],
                links: vec![],
            },
        ]
    );
    assert_eq!(
        l.html,
        "<h1 id=\"b0\">Title</h1>\n<p id=\"b1\">Hello <em>big</em> world</p>\n"
    );
}

#[test]
fn a_space_inside_a_link_stays_in_the_link_and_one_outside_does_not() {
    let l = layer("see [the **site**](https://x.example/a) now");
    assert_eq!(l.blocks.len(), 1);
    let b = &l.blocks[0];
    assert_eq!(b.text, "see the site now");
    assert_eq!(
        b.links,
        vec![LinkSpan {
            src: 4..12,
            href: "https://x.example/a".into()
        }]
    );
    // the space between "the" and "site" is in the link and takes what both
    // sides share (plain); the spaces around the link are plain and outside it
    assert_eq!(
        b.spans,
        vec![span(0, 8, PLAIN), span(8, 12, STRONG), span(12, 16, PLAIN)]
    );
    assert_eq!(
        l.html,
        "<p id=\"b0\">see <a href=\"https://x.example/a\">the <strong>site</strong></a> now</p>\n"
    );
}

#[test]
fn two_links_side_by_side_stay_two_links() {
    let l = layer("[a](/x)[b](/x)");
    assert_eq!(l.blocks[0].text, "ab");
    assert_eq!(
        l.blocks[0].links,
        vec![
            LinkSpan {
                src: 0..1,
                href: "/x".into()
            },
            LinkSpan {
                src: 1..2,
                href: "/x".into()
            }
        ]
    );
    assert_eq!(
        l.html,
        "<p id=\"b0\"><a href=\"/x\">a</a><a href=\"/x\">b</a></p>\n"
    );
}

#[test]
fn an_ordered_list_keeps_its_numbers_and_each_item_is_a_block() {
    let l = layer("3. a\n4. `b`\n");
    assert_eq!(
        l.blocks,
        vec![
            BlockRun {
                id: 0,
                role: Role::ListItem {
                    ordered: true,
                    number: 3
                },
                text: "a".into(),
                spans: vec![span(0, 1, PLAIN)],
                links: vec![]
            },
            BlockRun {
                id: 1,
                role: Role::ListItem {
                    ordered: true,
                    number: 4
                },
                text: "b".into(),
                spans: vec![span(0, 1, CODE)],
                links: vec![]
            },
        ]
    );
    assert_eq!(
        l.html,
        "<ol start=\"3\">\n<li id=\"b0\">a</li>\n<li id=\"b1\"><code>b</code></li>\n</ol>\n"
    );
}

#[test]
fn an_unordered_list_and_a_quote() {
    let l = layer("- x\n- y\n\n> q\n");
    let roles: Vec<Role> = l.blocks.iter().map(|b| b.role.clone()).collect();
    assert_eq!(
        roles,
        vec![
            Role::ListItem {
                ordered: false,
                number: 0
            },
            Role::ListItem {
                ordered: false,
                number: 0
            },
            Role::Quote
        ]
    );
    assert_eq!(l.html, "<ul>\n<li id=\"b0\">x</li>\n<li id=\"b1\">y</li>\n</ul>\n<blockquote id=\"b2\"><p>q</p></blockquote>\n");
}

#[test]
fn the_five_characters_are_escaped_in_html_and_kept_in_the_text() {
    let l = layer("a < b & \"c\" 'd' > e");
    assert_eq!(l.blocks[0].text, "a < b & \"c\" 'd' > e");
    assert_eq!(
        l.html,
        "<p id=\"b0\">a &lt; b &amp; &quot;c&quot; &#39;d&#39; &gt; e</p>\n"
    );
}

#[test]
fn a_code_block_keeps_its_lines_and_spaces() {
    let l = layer("```rust\nfn x() {}\n  y < z\n```\n");
    assert_eq!(
        l.blocks,
        vec![BlockRun {
            id: 0,
            role: Role::Code,
            text: "fn x() {}\n  y < z".into(),
            spans: vec![span(0, 17, CODE)],
            links: vec![]
        }]
    );
    assert_eq!(
        l.html,
        "<pre id=\"b0\"><code>fn x() {}\n  y &lt; z</code></pre>\n"
    );
}

#[test]
fn a_block_with_no_visible_text_is_left_out_and_ids_stay_dense() {
    let l = layer("#   \n\npara\n");
    assert_eq!(l.blocks.len(), 1);
    assert_eq!(l.blocks[0].id, 0);
    assert_eq!(l.html, "<p id=\"b0\">para</p>\n");
}

#[test]
fn a_code_block_of_blank_lines_is_left_out() {
    let l = layer("```\n   \n\t\n```\n\npara\n");
    assert_eq!(l.blocks.len(), 1);
    assert_eq!(l.blocks[0].role, Role::Paragraph);
    assert_eq!(l.html, "<p id=\"b0\">para</p>\n");
}

// ---- properties against text_lines ---------------------------------------

/// The text content of an HTML fragment this crate writes: tags dropped,
/// the five entities read back, newlines between elements dropped.
fn text_content(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn corpus() -> Vec<String> {
    let mut docs = vec![
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")).unwrap(),
        "# A *b* `c`\n\nOne  two\nthree [four **five** six](https://e.example) seven.\n\n- [x](/a) y\n- **z**\n\n1. one\n2. two\n\n> q *r* s\n\n```\n a  b\n\n c\n```\n".to_owned(),
        "&<>\"' **&** [<](/l?a=1&b=2)".to_owned(),
    ];
    docs.push(docs[1].repeat(20));
    docs
}

#[test]
fn every_block_text_is_what_text_lines_reads_and_the_html_says_the_same() {
    let mut compared = 0usize;
    for src in corpus() {
        let doc = parse(&src);
        let l = text_layer(&doc);
        // text_lines gives one entry per non-code block and one per non-blank code line
        let mut from_layer = Vec::new();
        for b in &l.blocks {
            if b.role == Role::Code {
                from_layer.extend(
                    b.text
                        .split('\n')
                        .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" ")),
                );
            } else {
                from_layer.push(b.text.clone());
            }
        }
        from_layer.retain(|s| !s.is_empty());
        assert_eq!(from_layer, doc.text_lines());
        let joined: String = l.blocks.iter().map(|b| b.text.replace('\n', "")).collect();
        assert_eq!(text_content(&l.html).replace('\n', ""), joined);
        compared += l.blocks.len();
    }
    assert!(compared > 0, "0 blocks compared");
}

#[test]
fn spans_cover_the_text_exactly_and_links_cut_whole_characters() {
    let mut compared = 0usize;
    for src in corpus() {
        for b in text_layer(&parse(&src)).blocks {
            let mut at = 0u32;
            for s in &b.spans {
                assert_eq!(s.src.start, at, "gap or overlap in {:?}", b.text);
                assert!(s.src.end > s.src.start);
                at = s.src.end;
            }
            assert_eq!(
                at as usize,
                b.text.len(),
                "spans end before the text in {:?}",
                b.text
            );
            for w in b.spans.windows(2) {
                assert_ne!(
                    w[0].style, w[1].style,
                    "two neighbouring spans with one style"
                );
            }
            for k in &b.links {
                assert!(
                    b.text.is_char_boundary(k.src.start as usize)
                        && b.text.is_char_boundary(k.src.end as usize)
                );
                assert!(!b.text[k.src.start as usize..k.src.end as usize].is_empty());
            }
            compared += 1;
        }
    }
    assert!(compared > 0, "0 blocks compared");
}

#[test]
fn ids_are_dense_in_reading_order() {
    for src in corpus() {
        let l = text_layer(&parse(&src));
        for (i, b) in l.blocks.iter().enumerate() {
            assert_eq!(b.id as usize, i);
            assert!(l.html.contains(&format!(" id=\"b{i}\"")));
        }
        assert!(!l.html.contains(&format!(" id=\"b{}\"", l.blocks.len())));
    }
}
