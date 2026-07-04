use std::fs;

#[test]
fn gitattributes_preserves_repo_line_ending_policy() {
    let root = env!("CARGO_MANIFEST_DIR");
    let attrs = fs::read_to_string(format!("{root}/.gitattributes"))
        .expect("Failed to read .gitattributes");

    let required = [
        "* text=auto eol=lf",
        "*.bat text eol=crlf",
        "*.cmd text eol=crlf",
        "*.ps1 text eol=crlf",
    ];

    for pattern in required {
        assert!(
            attrs.lines().any(|line| line.trim() == pattern),
            ".gitattributes must contain `{pattern}`"
        );
    }
}
