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

[Unreleased]: https://github.com/ext-sakamoro/ALICE-Markup/commits/main
