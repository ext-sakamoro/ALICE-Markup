# Changelog

All notable changes to this project are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- CI: 共有の `test_check_eol`、`eol.yml`、`public-text.yml` を新しい版にした
- CI: the shared public text workflow and its tests (`.github/workflows/public-text.yml`, `scripts/test_public_text.py`) are now byte-identical with the other ALICE crates' copies. The tests of `scripts/cargo_test_min.sh` move from the shared workflow to the `test` job of `ci.yml`, and `scripts/preflight.sh` is regenerated to match; the public text tests gain the cases the shared copy has (a planted finding, the checker's revision and checksum, settings approved by the pin)
- Tests of `scripts/cargo_test_min.sh` run on Windows too, now that they are in the `test` job of every OS: they start Git's bash (the `bash` on PATH there can be the WSL launcher), join PATH with the platform separator, and write the fake `cargo` with LF line endings

### Added

- `text_layer` / `TextLayer` / `BlockRun` / `Role` / `InlineStyle` / `StyleRange` / `LinkSpan`: 1 つの文書から、支援技術と検索向けの HTML の断片と、block ごとに組版へ渡す文字 (空白は `text_lines` と同じく 1 つに、code block は書いたまま)・書式の範囲 (隙間・重なりなし)・link の範囲を出す block の要素は id `b<n>` を持ち、HTML と block の文字は同じ 2 つの run の間の空白は両方に共通の書式を取り、両側が同じ link の時だけその link に入る `& < > " '` を escape する 文字が空の block は出さない 試験 `tests/text_layer.rs` (手で書いた期待値 9 本、`text_lines` との突合と範囲の性質 3 本)
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
