//! The README's first Rust example is the crate's doctest (README text is not
//! compiled, the doctest is): the two must be the same lines.

/// The memory ceiling (see `alice_markup_test_bounds`): parsing the inputs
/// these tests use (at most 512 KiB each) never holds more than 64 MiB at
/// once (the whole suite peaks at 1.6 MiB, in `tests/properties.rs` on one
/// thread). A parser that keeps allocating without advancing fails here.
#[global_allocator]
static BUDGET: alice_markup_test_bounds::Budget = alice_markup_test_bounds::Budget::new(64 << 20);

fn readme_example() -> Vec<String> {
    let readme = include_str!("../README.md");
    let start = readme
        .find("```rust\n")
        .expect("README has a ```rust block")
        + "```rust\n".len();
    let end = start + readme[start..].find("```").expect("the block is closed");
    readme[start..end].lines().map(str::to_string).collect()
}

fn doctest_example() -> Vec<String> {
    let lib = include_str!("../src/lib.rs");
    let doc: Vec<&str> = lib
        .lines()
        .take_while(|l| l.starts_with("//!"))
        .map(|l| {
            l.strip_prefix("//! ")
                .or_else(|| l.strip_prefix("//!"))
                .unwrap_or(l)
        })
        .collect();
    let open = doc
        .iter()
        .position(|l| *l == "```")
        .expect("the crate doc has a code block");
    let close = open
        + 1
        + doc[open + 1..]
            .iter()
            .position(|l| *l == "```")
            .expect("closed");
    doc[open + 1..close]
        .iter()
        .map(|l| (*l).to_string())
        .collect()
}

#[test]
fn readme_example_is_the_doctest() {
    let readme = readme_example();
    assert!(
        readme.len() >= 3,
        "README example has {} lines",
        readme.len()
    );
    assert_eq!(readme, doctest_example());
}
