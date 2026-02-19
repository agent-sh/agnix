//! Fuzz target for Markdown parsing
//!
//! This target tests the XML tag and import extraction functions from
//! markdown content. These functions use regex internally and must handle
//! arbitrary input without panicking or producing invalid output.
//!
//! # Byte offset invariants
//!
//! The parsing functions sanitize their input (normalizing line endings and
//! stripping C0 control characters) before passing content to `pulldown-cmark`.
//! Returned byte offsets are therefore relative to the *sanitized* content, not
//! the raw fuzzer input. We pre-sanitize here so that the invariant assertions
//! check offsets against the same string the functions operated on.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    // Pre-sanitize so that byte-offset invariant checks below use the same
    // string the parsing functions will operate on internally.
    let sanitized = agnix_core::__internal::sanitize_for_pulldown_cmark(data);

    // Test extract_xml_tags() - should never panic
    let tags = agnix_core::__internal::extract_xml_tags(data);

    // Verify invariants for XML tags:
    for tag in &tags {
        // Byte offsets must be within bounds of the sanitized content
        assert!(tag.start_byte <= sanitized.len());
        assert!(tag.end_byte <= sanitized.len());
        assert!(tag.start_byte <= tag.end_byte);

        // UTF-8 boundary validation: byte offsets must be at character boundaries
        assert!(sanitized.is_char_boundary(tag.start_byte), "start_byte must be at UTF-8 boundary");
        assert!(sanitized.is_char_boundary(tag.end_byte), "end_byte must be at UTF-8 boundary");

        // Line/column must be positive (1-indexed)
        assert!(tag.line >= 1);
        assert!(tag.column >= 1);
    }

    // Test check_xml_balance() - should never panic
    let _errors = agnix_core::__internal::check_xml_balance(&tags);
    let _errors_with_end =
        agnix_core::__internal::check_xml_balance_with_content_end(&tags, Some(sanitized.len()));

    // Test extract_imports() - should never panic
    let imports = agnix_core::__internal::extract_imports(data);

    // Verify invariants for imports:
    for import in &imports {
        // Byte offsets must be within bounds of the sanitized content
        assert!(import.start_byte <= sanitized.len());
        assert!(import.end_byte <= sanitized.len());
        assert!(import.start_byte <= import.end_byte);

        // UTF-8 boundary validation
        assert!(sanitized.is_char_boundary(import.start_byte), "start_byte must be at UTF-8 boundary");
        assert!(sanitized.is_char_boundary(import.end_byte), "end_byte must be at UTF-8 boundary");

        // Line/column must be positive (1-indexed)
        assert!(import.line >= 1);
        assert!(import.column >= 1);
    }

    // Test extract_markdown_links() - should never panic
    let links = agnix_core::__internal::extract_markdown_links(data);

    // Verify invariants for links:
    for link in &links {
        // Byte offsets must be within bounds of the sanitized content
        assert!(link.start_byte <= sanitized.len());
        assert!(link.end_byte <= sanitized.len());
        assert!(link.start_byte <= link.end_byte);

        // UTF-8 boundary validation
        assert!(sanitized.is_char_boundary(link.start_byte), "start_byte must be at UTF-8 boundary");
        assert!(sanitized.is_char_boundary(link.end_byte), "end_byte must be at UTF-8 boundary");

        // Line/column must be positive (1-indexed)
        assert!(link.line >= 1);
        assert!(link.column >= 1);
    }
});
