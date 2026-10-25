# ALICE-Markup

A small, deterministic Markdown subset for Rust: `no_std` (with `alloc`), no dependencies, no `unsafe`.

```rust
use alice_markup::{parse, split_more};

let text = "# Notes\n\nOpen to everyone.\n<!-- more -->\nThe rest, with a [link](https://example.org).";
let (first, rest) = split_more(text).unwrap();
assert_eq!(parse(first).text_lines(), ["Notes", "Open to everyone."]);
assert_eq!(parse(rest).text_lines(), ["The rest, with a link."]);
```

## What it is not for

It is not a CommonMark implementation: tables, raw HTML and reference links are read as plain text, and a nested list item becomes one more item of its list.
It is for short texts written for it, such as articles on a site, where a fixed, small subset and a guaranteed reader text matter more than coverage.

## What it reads

| Block | Syntax |
|---|---|
| Heading | `#` to `######`, then a space |
| Paragraph | lines that start no other block, joined with a space |
| List | `- ` / `* ` / `+ ` items, or `1. ` / `1) ` items (an indented line continues an item) |
| Quote | lines starting with `>` |
| Code | fenced with three or more backticks, kept verbatim |

| Inline | Syntax |
|---|---|
| Emphasis / strong | `*a*` `_a_` / `**a**` `__a__` |
| Code span | `` `code` `` |
| Link | `[label](target)`: only `http(s)://`, `mailto:`, `/path` and `#fragment` targets become links; any other target keeps just the label |
| Image | `![alt](src)` is read as its alt text |
| Escape | a backslash before ASCII punctuation |

Everything else (HTML, tables, setext headings, reference links, hard breaks) is plain text. Nothing is ever dropped silently, and no input is an error.

## Guarantees

- The same input gives the same document on every target (byte and `char` comparisons only).
- `parse` never panics, and its time is linear in the input length.
  A closing delimiter is looked for at most 4096 bytes ahead, and nesting is limited to 16 levels.
- `Document::text_lines` is what a reader sees, one line per heading, paragraph, quote, list item and code line, with whitespace collapsed.
  A renderer that shows these strings and a check that looks for a text in a built site compare the same thing.
- `split_more` finds exactly one `<!-- more -->` line, or returns an error.

## Tests

- `tests/oracle.rs`: each rule as hand-written expected documents.
- `tests/properties.rs`: generated inputs with a fixed seed. They check determinism, link targets, reader lines and linear time.
- `fuzz/`: `parse` and `split_more` under cargo-fuzz.

## License

`LicenseRef-Proprietary`. The text is [LICENSE-Proprietary](LICENSE-Proprietary).

Enquiries: https://extoria.co.jp/
