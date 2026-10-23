# Changelog

All notable changes to this project are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- CI: the shared public text workflow and its tests (`.github/workflows/public-text.yml`, `scripts/test_public_text.py`) are now byte-identical with the other ALICE crates' copies. The tests of `scripts/cargo_test_min.sh` move from the shared workflow to the `test` job of `ci.yml`, and `scripts/preflight.sh` is regenerated to match; the public text tests gain the cases the shared copy has (a planted finding, the checker's revision and checksum, settings approved by the pin)

### Added

- `parse`: headings, paragraphs, lists, quotes and fenced code; emphasis, strong emphasis, code spans, links and backslash escapes
- `Document::text_lines`: the text a reader sees, one line per block line
- `is_safe_href`: the link targets that become links (`http(s)://`, `mailto:`, `/path`, `#fragment`)
- `split_more`: the text before and after the one `<!-- more -->` line
- Oracle tests, generated-input properties (determinism, link targets, linear time) and two fuzz targets
- Test bounds: every test runs under a 64 MiB ceiling on live bytes and each parse under a 10 s deadline, so a parser that stops advancing fails its test instead of filling memory or running on (test-only dev-dependency `tests/bounds`, not published)
- Text files are stored and checked out with LF on every OS (`.gitattributes`), and the line-endings workflow (`scripts/check_eol.py`, the ALICE shared copy) checks it on all three OS; the README test reads the README line by line, so a CRLF checkout holds the same lines
- CI runs the tests through `scripts/cargo_test_min.sh`, which fails unless at least 64 tests passed (a run that loses a test file or executes nothing no longer passes)
- Oracle test: a closing delimiter run longer than the opening one does not close it
- Oracle tests for the 4096-byte closer window, escaped closers, code spans inside emphasis, delimiter neighbours, all-space code spans, a list interrupting a paragraph, and unit tests of the nesting limit at the recursion

[Unreleased]: https://github.com/ext-sakamoro/ALICE-Markup/commits/main
