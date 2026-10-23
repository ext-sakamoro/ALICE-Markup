//! Properties over generated inputs (a fixed-seed xorshift, so every run sees
//! the same inputs): no panic, the same document twice, links only to accepted
//! targets, the reader text holds no markup a renderer would interpret, and
//! the time stays linear in the input length.

/// The memory ceiling (see `alice_markup_test_bounds`): parsing the inputs
/// these tests use (at most 512 KiB each) never holds more than 64 MiB at
/// once (the whole suite peaks at 1.6 MiB, in `tests/properties.rs` on one
/// thread). A parser that keeps allocating without advancing fails here.
#[global_allocator]
static BUDGET: alice_markup_test_bounds::Budget = alice_markup_test_bounds::Budget::new(64 << 20);

/// The parser under a deadline (see `alice_markup_test_bounds::within`): every
/// input here parses in well under a second, so a parser that spins without
/// advancing fails the test at the deadline.
fn parse(src: &str) -> Document {
    let src = src.to_owned();
    alice_markup_test_bounds::within(std::time::Duration::from_secs(10), move || {
        alice_markup::parse(&src)
    })
}

use alice_markup::{is_safe_href, split_more, Block, Document, Inline};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % (n as u64)).unwrap_or(0)
    }
}

/// Pieces that exercise every rule, mixed at random.
const PIECES: &[&str] = &[
    "#",
    "## ",
    "###### ",
    "####### ",
    "- ",
    "* ",
    "+ ",
    "1. ",
    "12) ",
    "> ",
    "```",
    "````",
    "`",
    "``",
    "*",
    "**",
    "***",
    "_",
    "__",
    "[",
    "]",
    "(",
    ")",
    "![",
    "\\",
    "\\*",
    "\n",
    "\n\n",
    "\r\n",
    "\r",
    " ",
    "    ",
    "\t",
    "a",
    "word",
    "日本語",
    "é",
    "<!-- more -->",
    "https://example.org",
    "/path",
    "javascript:x",
    "//host",
    "#frag",
    "mailto:a@b.c",
    "<b>",
    "&amp;",
    "[x](https://example.org)",
    "[*y*](/a)",
    "[z](javascript:x)",
    "![alt](/i.png)",
];

fn generate(rng: &mut Rng, pieces: usize) -> String {
    (0..pieces)
        .map(|_| PIECES[rng.below(PIECES.len())])
        .collect()
}

fn walk_inlines(inlines: &[Inline], f: &mut impl FnMut(&Inline)) {
    for inline in inlines {
        f(inline);
        match inline {
            Inline::Emphasis(inner) | Inline::Strong(inner) => walk_inlines(inner, f),
            Inline::Link { label, .. } => walk_inlines(label, f),
            Inline::Text(_) | Inline::Code(_) => {}
        }
    }
}

fn walk(doc: &Document, mut f: impl FnMut(&Inline)) {
    for block in &doc.blocks {
        match block {
            Block::Heading { inlines, .. } | Block::Paragraph(inlines) | Block::Quote(inlines) => {
                walk_inlines(inlines, &mut f);
            }
            Block::List { items, .. } => items.iter().for_each(|item| walk_inlines(item, &mut f)),
            Block::Code { .. } => {}
        }
    }
}

#[test]
fn generated_inputs_parse_the_same_twice_and_link_safely() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut links = 0usize;
    let mut blocks = 0usize;
    for _ in 0..20_000 {
        let pieces = 1 + rng.below(40);
        let src = generate(&mut rng, pieces);
        let doc = parse(&src);
        assert_eq!(doc, parse(&src), "not deterministic: {src:?}");
        blocks += doc.blocks.len();
        walk(&doc, |inline| {
            match inline {
                Inline::Link { href, label } => {
                    links += 1;
                    assert!(is_safe_href(href), "unsafe href {href:?} from {src:?}");
                    // a label holds no link
                    walk_inlines(label, &mut |i| {
                        assert!(!matches!(i, Inline::Link { .. }), "nested link in {src:?}");
                    });
                }
                Inline::Text(t) => assert!(!t.is_empty(), "empty text node from {src:?}"),
                Inline::Emphasis(inner) | Inline::Strong(inner) => {
                    assert!(!inner.is_empty(), "empty emphasis from {src:?}");
                }
                Inline::Code(_) => {}
            }
        });
        for line in doc.text_lines() {
            assert!(
                !line.is_empty()
                    && line.trim() == line
                    && !line.contains("  ")
                    && !line.contains('\n'),
                "{line:?}"
            );
        }
        if let Some(level) = doc.blocks.iter().find_map(|b| match b {
            Block::Heading { level, .. } => Some(*level),
            _ => None,
        }) {
            assert!((1..=6).contains(&level));
        }
        let _ = split_more(&src);
    }
    // the generator really reaches links and blocks (a vacuous run fails)
    assert!(links > 100, "only {links} links generated");
    assert!(blocks > 10_000, "only {blocks} blocks generated");
}

#[test]
fn every_text_character_survives_when_nothing_is_markup() {
    // plain words and spaces: the reader text is the input with whitespace collapsed
    let mut rng = Rng(42);
    for _ in 0..2_000 {
        let words: Vec<&str> = (0..=rng.below(20))
            .map(|_| ["alpha", "beta", "日本", "x1", "é"][rng.below(5)])
            .collect();
        let src = words.join(" ");
        assert_eq!(parse(&src).text_lines(), vec![src.clone()]);
    }
}

#[test]
fn split_more_round_trip() {
    let mut rng = Rng(7);
    for _ in 0..2_000 {
        let (nb, na) = (rng.below(10), rng.below(10));
        let before = generate(&mut rng, nb).replace("<!-- more -->", "");
        let after = generate(&mut rng, na).replace("<!-- more -->", "");
        let before = if before.is_empty() || before.ends_with('\n') {
            before
        } else {
            before + "\n"
        };
        let text = format!("{before}<!-- more -->\n{after}");
        // a marker split across pieces could form a second marker; skip those
        if text.matches("<!-- more -->").count() != 1 {
            continue;
        }
        assert_eq!(
            split_more(&text),
            Ok((before.as_str(), after.as_str())),
            "{text:?}"
        );
    }
}

/// Inputs that would be quadratic for a naive closer search: many openers that
/// never close. The work per byte stays bounded, so 64 times the input takes
/// well under 64 times 64 the time.
#[test]
// the wall clock is the point of this test (it measures time), not a result of the parser
#[allow(clippy::disallowed_methods)]
fn pathological_inputs_stay_linear() {
    let cases: [fn(usize) -> String; 5] = [
        |n| "*a ".repeat(n),
        |n| "[".repeat(n),
        |n| "`".repeat(n) + "x",
        |n| "_a".repeat(n),
        |n| "[a](".repeat(n),
    ];
    for (k, make) in cases.iter().enumerate() {
        let small = make(2_000);
        let big = make(128_000);
        // this test bounds its own time (the ratio below), and a debug build
        // takes longer than the per-call deadline on 128k pieces, so it calls
        // the parser directly
        let t0 = std::time::Instant::now();
        let _ = alice_markup::parse(&small);
        let t_small = t0.elapsed().as_secs_f64().max(1e-4);
        let t1 = std::time::Instant::now();
        let doc = alice_markup::parse(&big);
        let t_big = t1.elapsed().as_secs_f64();
        assert!(!doc.blocks.is_empty());
        // linear would be 64x; quadratic 4096x
        assert!(
            t_big < t_small * 400.0 + 2.0,
            "case {k}: {t_small} s for 2k pieces, {t_big} s for 128k"
        );
    }
}
