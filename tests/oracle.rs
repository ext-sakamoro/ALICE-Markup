//! The subset, case by case: each test writes the expected document by hand
//! from the rules in the crate documentation (never from the parser's output).

use alice_markup::{is_safe_href, parse, split_more, Block, Document, Inline, MoreError};

fn t(s: &str) -> Inline {
    Inline::Text(s.to_string())
}

fn para(inlines: Vec<Inline>) -> Document {
    Document {
        blocks: vec![Block::Paragraph(inlines)],
    }
}

// ---- blocks ---------------------------------------------------------------

#[test]
fn empty_and_blank_input_has_no_blocks() {
    assert_eq!(parse(""), Document::default());
    assert_eq!(parse("\n\n   \n\t\n"), Document::default());
}

#[test]
fn headings_of_each_level() {
    for level in 1..=6u8 {
        let src = format!("{} Title", "#".repeat(level.into()));
        assert_eq!(
            parse(&src).blocks,
            vec![Block::Heading {
                level,
                inlines: vec![t("Title")]
            }],
            "{src}"
        );
    }
}

#[test]
fn seven_hashes_or_no_space_is_a_paragraph() {
    assert_eq!(parse("####### x"), para(vec![t("####### x")]));
    assert_eq!(parse("#hashtag"), para(vec![t("#hashtag")]));
}

#[test]
fn heading_closing_hashes_are_dropped() {
    assert_eq!(
        parse("## Title ##").blocks,
        vec![Block::Heading {
            level: 2,
            inlines: vec![t("Title")]
        }]
    );
    // a hash run not preceded by a space is part of the text
    assert_eq!(
        parse("## C#").blocks,
        vec![Block::Heading {
            level: 2,
            inlines: vec![t("C#")]
        }]
    );
}

#[test]
fn empty_heading() {
    assert_eq!(
        parse("#").blocks,
        vec![Block::Heading {
            level: 1,
            inlines: vec![]
        }]
    );
}

#[test]
fn up_to_three_leading_spaces_before_a_marker() {
    assert_eq!(
        parse("   # Title").blocks,
        vec![Block::Heading {
            level: 1,
            inlines: vec![t("Title")]
        }]
    );
    // four spaces: not a heading (no indented code either): a paragraph
    assert_eq!(parse("    # Title"), para(vec![t("# Title")]));
}

#[test]
fn paragraph_lines_join_with_one_space() {
    assert_eq!(
        parse("one\ntwo  \n   three"),
        para(vec![t("one two three")])
    );
}

#[test]
fn blank_line_separates_paragraphs() {
    assert_eq!(
        parse("one\n\ntwo").blocks,
        vec![
            Block::Paragraph(vec![t("one")]),
            Block::Paragraph(vec![t("two")])
        ]
    );
}

#[test]
fn crlf_and_cr_are_line_breaks() {
    assert_eq!(parse("a\r\nb\rc"), para(vec![t("a b c")]));
    assert_eq!(parse("# T\r\n\r\nx").blocks.len(), 2);
}

#[test]
fn a_heading_interrupts_a_paragraph() {
    assert_eq!(
        parse("text\n# H\nmore").blocks,
        vec![
            Block::Paragraph(vec![t("text")]),
            Block::Heading {
                level: 1,
                inlines: vec![t("H")]
            },
            Block::Paragraph(vec![t("more")]),
        ]
    );
}

#[test]
fn unordered_list_with_each_bullet() {
    let want = Block::List {
        ordered: false,
        start: 0,
        items: vec![vec![t("a")], vec![t("b")], vec![t("c")]],
    };
    assert_eq!(parse("- a\n* b\n+ c").blocks, vec![want]);
}

#[test]
fn ordered_list_keeps_its_start() {
    let want = Block::List {
        ordered: true,
        start: 3,
        items: vec![vec![t("x")], vec![t("y")]],
    };
    assert_eq!(parse("3. x\n4) y").blocks, vec![want]);
}

#[test]
fn ten_digits_is_not_a_list_item() {
    assert_eq!(parse("1234567890. x"), para(vec![t("1234567890. x")]));
}

#[test]
fn list_item_continuation_needs_an_indent() {
    let want = Block::List {
        ordered: false,
        start: 0,
        items: vec![vec![t("first line second line")]],
    };
    assert_eq!(parse("- first line\n  second line").blocks, vec![want]);
    // without an indent the line ends the list and starts a paragraph
    assert_eq!(
        parse("- item\nafter").blocks,
        vec![
            Block::List {
                ordered: false,
                start: 0,
                items: vec![vec![t("item")]]
            },
            Block::Paragraph(vec![t("after")])
        ]
    );
}

#[test]
fn blank_lines_between_items_of_one_kind_keep_one_list() {
    let want = Block::List {
        ordered: false,
        start: 0,
        items: vec![vec![t("a")], vec![t("b")]],
    };
    assert_eq!(parse("- a\n\n- b").blocks, vec![want]);
}

#[test]
fn a_change_of_kind_starts_a_new_list() {
    assert_eq!(
        parse("- a\n1. b").blocks,
        vec![
            Block::List {
                ordered: false,
                start: 0,
                items: vec![vec![t("a")]]
            },
            Block::List {
                ordered: true,
                start: 1,
                items: vec![vec![t("b")]]
            },
        ]
    );
}

#[test]
fn a_bare_marker_is_an_empty_item_but_a_rule_is_text() {
    assert_eq!(
        parse("-").blocks,
        vec![Block::List {
            ordered: false,
            start: 0,
            items: vec![vec![]]
        }]
    );
    assert_eq!(parse("---"), para(vec![t("---")]));
    assert_eq!(parse("***"), para(vec![t("***")]));
}

#[test]
fn quote_lines_join() {
    assert_eq!(
        parse("> one\n>two\n> three").blocks,
        vec![Block::Quote(vec![t("one two three")])]
    );
    assert_eq!(
        parse("> q\nnot").blocks,
        vec![Block::Quote(vec![t("q")]), Block::Paragraph(vec![t("not")])]
    );
}

#[test]
fn fenced_code_is_verbatim() {
    let src = "```rust\nlet a = *b*;\n  [x](y)\n```\nafter";
    assert_eq!(
        parse(src).blocks,
        vec![
            Block::Code {
                info: "rust".into(),
                text: "let a = *b*;\n  [x](y)".into()
            },
            Block::Paragraph(vec![t("after")]),
        ]
    );
}

#[test]
fn fence_needs_at_least_as_many_backticks_to_close() {
    let src = "````\n```\n````";
    assert_eq!(
        parse(src).blocks,
        vec![Block::Code {
            info: String::new(),
            text: "```".into()
        }]
    );
}

#[test]
fn unclosed_fence_runs_to_the_end() {
    assert_eq!(
        parse("```\na\n\nb").blocks,
        vec![Block::Code {
            info: String::new(),
            text: "a\n\nb".into()
        }]
    );
}

#[test]
fn fence_info_takes_the_first_word() {
    assert_eq!(
        parse("``` sh  extra\nx\n```").blocks,
        vec![Block::Code {
            info: "sh".into(),
            text: "x".into()
        }]
    );
}

#[test]
fn backticks_in_the_info_make_it_text() {
    assert_eq!(parse("``` a`b"), para(vec![t("``` a`b")]));
}

#[test]
fn html_is_text() {
    assert_eq!(
        parse("<script>alert(1)</script>"),
        para(vec![t("<script>alert(1)</script>")])
    );
}

// ---- inlines --------------------------------------------------------------

#[test]
fn emphasis_and_strong() {
    assert_eq!(
        parse("a *b* c"),
        para(vec![t("a "), Inline::Emphasis(vec![t("b")]), t(" c")])
    );
    assert_eq!(
        parse("a _b_ c"),
        para(vec![t("a "), Inline::Emphasis(vec![t("b")]), t(" c")])
    );
    assert_eq!(parse("**b**"), para(vec![Inline::Strong(vec![t("b")])]));
    assert_eq!(parse("__b__"), para(vec![Inline::Strong(vec![t("b")])]));
}

#[test]
fn nested_emphasis() {
    assert_eq!(
        parse("**a *b* c**"),
        para(vec![Inline::Strong(vec![
            t("a "),
            Inline::Emphasis(vec![t("b")]),
            t(" c")
        ])])
    );
}

#[test]
fn unmatched_or_spaced_delimiters_are_text() {
    assert_eq!(parse("a * b * c"), para(vec![t("a * b * c")]));
    assert_eq!(parse("*open"), para(vec![t("*open")]));
    assert_eq!(
        parse("2*3*4"),
        para(vec![t("2"), Inline::Emphasis(vec![t("3")]), t("4")])
    );
    // underscores inside a word are text
    assert_eq!(parse("snake_case_name"), para(vec![t("snake_case_name")]));
    // three or more in a run are text
    assert_eq!(parse("***x***"), para(vec![t("***x***")]));
}

#[test]
fn emphasis_does_not_cross_a_paragraph() {
    assert_eq!(
        parse("*a\n\nb*").blocks,
        vec![
            Block::Paragraph(vec![t("*a")]),
            Block::Paragraph(vec![t("b*")])
        ]
    );
}

#[test]
fn code_spans() {
    assert_eq!(
        parse("use `a*b*` here"),
        para(vec![t("use "), Inline::Code("a*b*".into()), t(" here")])
    );
    assert_eq!(parse("`` a`b ``"), para(vec![Inline::Code("a`b".into())]));
    assert_eq!(parse("`unclosed"), para(vec![t("`unclosed")]));
}

#[test]
fn code_span_line_break_is_a_space() {
    assert_eq!(parse("`a\nb`"), para(vec![Inline::Code("a b".into())]));
}

#[test]
fn backslash_escapes_punctuation_only() {
    assert_eq!(parse(r"\*not\* \[x\] \a"), para(vec![t(r"*not* [x] \a")]));
}

#[test]
fn links() {
    assert_eq!(
        parse("see [the *docs*](https://example.org/a?b=1)."),
        para(vec![
            t("see "),
            Inline::Link {
                label: vec![t("the "), Inline::Emphasis(vec![t("docs")])],
                href: "https://example.org/a?b=1".into()
            },
            t("."),
        ])
    );
    assert_eq!(
        parse("[home](/)"),
        para(vec![Inline::Link {
            label: vec![t("home")],
            href: "/".into()
        }])
    );
}

#[test]
fn unsafe_link_targets_keep_the_label_only() {
    assert_eq!(parse("[x](javascript:alert(1))"), para(vec![t("x)")]));
    assert_eq!(parse("[x](javascript:void)"), para(vec![t("x")]));
    assert_eq!(parse("[x](//evil.example)"), para(vec![t("x")]));
    assert_eq!(parse("[x](data:text/html,hi)"), para(vec![t("x")]));
}

#[test]
fn link_syntax_errors_are_text() {
    assert_eq!(parse("[x] (y)"), para(vec![t("[x] (y)")]));
    assert_eq!(parse("[x](a b)"), para(vec![t("[x](a b)")]));
    assert_eq!(parse("[x"), para(vec![t("[x")]));
}

#[test]
fn links_do_not_nest() {
    assert_eq!(
        parse("[a [b](/x) c](/y)"),
        para(vec![
            t("[a "),
            Inline::Link {
                label: vec![t("b")],
                href: "/x".into()
            },
            t(" c](/y)")
        ])
    );
}

#[test]
fn images_become_their_alt_text() {
    assert_eq!(
        parse("![a cat](/cat.png) sits"),
        para(vec![t("a cat sits")])
    );
}

// ---- reader text ----------------------------------------------------------

#[test]
fn text_lines_are_what_a_reader_sees() {
    let src = "# The *Title*\n\nA [link](/a) and `code`.\n\n- one\n- **two**\n\n> quoted\n\n```\nlet  a = 1;\n\n  b\n```";
    assert_eq!(
        parse(src).text_lines(),
        [
            "The Title",
            "A link and code.",
            "one",
            "two",
            "quoted",
            "let a = 1;",
            "b"
        ]
    );
}

#[test]
fn text_lines_collapse_whitespace_and_skip_empty() {
    assert_eq!(parse("-\n- a\tb").text_lines(), ["a b"]);
    assert_eq!(parse("`  x  `").text_lines(), ["x"]);
}

// ---- the more marker ------------------------------------------------------

#[test]
fn split_more_cuts_at_the_marker_line() {
    assert_eq!(
        split_more("a\nb\n<!-- more -->\nc\n"),
        Ok(("a\nb\n", "c\n"))
    );
    assert_eq!(split_more("a\n  <!-- more -->  \nc"), Ok(("a\n", "c")));
    assert_eq!(split_more("<!-- more -->"), Ok(("", "")));
}

#[test]
fn split_more_needs_exactly_one_marker_on_its_own_line() {
    assert_eq!(split_more("a\nb"), Err(MoreError::Missing));
    assert_eq!(
        split_more("text <!-- more --> inline"),
        Err(MoreError::Missing)
    );
    assert_eq!(
        split_more("<!-- more -->\nx\n<!-- more -->"),
        Err(MoreError::Repeated(2))
    );
}

#[test]
fn split_more_ignores_tabs_and_carriage_returns_around_the_marker() {
    assert_eq!(
        split_more("a\r\n\t<!-- more -->\t\r\nb"),
        Ok(("a\r\n", "b"))
    );
    assert_eq!(split_more("a\n<!-- more -->\r"), Ok(("a\n", "")));
}

#[test]
fn more_errors_say_what_is_wrong() {
    assert_eq!(MoreError::Missing.to_string(), "no line is <!-- more -->");
    assert_eq!(
        MoreError::Repeated(3).to_string(),
        "3 lines are <!-- more -->, expected exactly one"
    );
}

// ---- href policy ----------------------------------------------------------

#[test]
fn href_policy() {
    for ok in [
        "https://a.b",
        "HTTP://a",
        "mailto:x@y.z",
        "/",
        "/a/b?c",
        "#top",
    ] {
        assert!(is_safe_href(ok), "{ok}");
    }
    for bad in [
        "",
        "javascript:x",
        "JavaScript:x",
        "data:x",
        "//host",
        "relative/path",
        "https://",
        "mailto:",
        "/a b",
        "/a\nb",
        "vbscript:x",
    ] {
        assert!(!is_safe_href(bad), "{bad}");
    }
}
