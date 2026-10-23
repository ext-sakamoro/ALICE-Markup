# Changelog

All notable changes to this project are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `parse`: headings, paragraphs, lists, quotes and fenced code; emphasis, strong emphasis, code spans, links and backslash escapes
- `Document::text_lines`: the text a reader sees, one line per block line
- `is_safe_href`: the link targets that become links (`http(s)://`, `mailto:`, `/path`, `#fragment`)
- `split_more`: the text before and after the one `<!-- more -->` line
- Oracle tests, generated-input properties (determinism, link targets, linear time) and two fuzz targets
- Test bounds: every test runs under a 64 MiB ceiling on live bytes and each parse under a 10 s deadline, so a parser that stops advancing fails its test instead of filling memory or running on (test-only dev-dependency `tests/bounds`, not published)
- CI runs the tests through `scripts/cargo_test_min.sh`, which fails unless at least 64 tests passed (a run that loses a test file or executes nothing no longer passes)
- Oracle test: a closing delimiter run longer than the opening one does not close it
- Oracle tests for the 4096-byte closer window, escaped closers, code spans inside emphasis, delimiter neighbours, all-space code spans, a list interrupting a paragraph, and unit tests of the nesting limit at the recursion

[Unreleased]: https://github.com/ext-sakamoro/ALICE-Markup/commits/main
