#![no_main]
//! split_more never panics, and a split puts the text back together with the
//! marker line between the two parts.

use alice_markup::{split_more, MORE_MARKER};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else { return };
    if let Ok((before, after)) = split_more(text) {
        assert!(text.starts_with(before) && text.ends_with(after));
        let middle = &text[before.len()..text.len() - after.len()];
        assert_eq!(middle.trim_matches(|c| c == ' ' || c == '\t' || c == '\r' || c == '\n'), MORE_MARKER);
    }
});
