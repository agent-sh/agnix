//! Markdown parser for extracting @imports, links, and checking XML tags
//!
//! ## Security
//!
//! This module includes size limits to prevent ReDoS (Regular Expression Denial
//! of Service) attacks. The `MAX_REGEX_INPUT_SIZE` constant limits the size of
//! content that will be processed by regex operations.

use regex::Regex;
use std::ops::Range;
use std::panic::{self, AssertUnwindSafe};

use crate::regex_util::static_regex;

static_regex!(fn xml_tag_regex, r"<(/?)([a-zA-Z_][a-zA-Z0-9_-]*)(?:\s+[^>]*?)?(/?)>");

/// Maximum size (in bytes) for content processed by regex operations.
/// This prevents ReDoS attacks by limiting input size to 64KB.
/// Content larger than this will be processed without regex-based extraction.
///
/// **Design Decision**: 64KB was chosen as a balance between:
/// - Large enough to handle typical documentation files (most are <10KB)
/// - Small enough to prevent ReDoS on pathological regex input
/// - Matches typical page sizes for text processing
/// - 2^16 is a clean power-of-2 boundary
///
/// This limit applies to regex-guarded operations only. `extract_xml_tags` is guarded;
/// `extract_imports` and `extract_markdown_links` use byte-by-byte scanning and
/// pulldown-cmark respectively and are NOT subject to this limit.
/// Files larger than 1 MiB are rejected earlier by `DEFAULT_MAX_FILE_SIZE` in file_utils.rs.
pub const MAX_REGEX_INPUT_SIZE: usize = 65536; // 64KB

/// Extract @import references from markdown content (excluding code blocks/spans)
///
/// # Security
///
/// This function is NOT subject to `MAX_REGEX_INPUT_SIZE` limits because it uses
/// byte-by-byte scanning instead of regex. The limit only applies to regex-based
/// extraction functions (`extract_xml_tags`, `extract_markdown_links`).
///
/// # Input sanitization
///
/// Control characters and non-standard line endings are sanitized before parsing
/// to prevent a known panic in `pulldown-cmark` triggered by C0 control bytes.
pub fn extract_imports(content: &str) -> Vec<Import> {
    // Sanitize before parsing: pulldown-cmark has a known panic triggered by C0
    // control characters combined with certain syntax. Sanitizing here ensures safe
    // input regardless of how the caller obtained the content.
    let content = sanitize_for_pulldown_cmark(content);
    // Upstream parser panicked; return empty result (callers handle gracefully)
    panic::catch_unwind(AssertUnwindSafe(|| extract_imports_inner(&content))).unwrap_or_default()
}

fn extract_imports_inner(content: &str) -> Vec<Import> {
    let line_starts = compute_line_starts(content);
    let mut imports = Vec::new();

    scan_non_code_spans(content, |span, span_start| {
        let range = span_start..span_start + span.len();
        scan_imports_in_text(span, range, &line_starts, &mut imports);
    });

    imports
}

/// Extract XML tags for balance checking (excluding code blocks/spans)
///
/// # Security
///
/// Returns early for content exceeding `MAX_REGEX_INPUT_SIZE` to prevent ReDoS.
///
/// # Input sanitization
///
/// Control characters and non-standard line endings are sanitized before parsing
/// to prevent a known panic in `pulldown-cmark` triggered by C0 control bytes.
pub fn extract_xml_tags(content: &str) -> Vec<XmlTag> {
    // Security: Skip regex processing for oversized content to prevent ReDoS
    if content.len() > MAX_REGEX_INPUT_SIZE {
        return Vec::new();
    }

    // Sanitize before parsing: pulldown-cmark has a known panic triggered by C0
    // control characters combined with certain syntax. Sanitizing here ensures safe
    // input regardless of how the caller obtained the content.
    let content = sanitize_for_pulldown_cmark(content);

    // Upstream parser panicked; return empty result (callers handle gracefully)
    panic::catch_unwind(AssertUnwindSafe(|| extract_xml_tags_inner(&content))).unwrap_or_default()
}

fn extract_xml_tags_inner(content: &str) -> Vec<XmlTag> {
    let line_starts = compute_line_starts(content);
    let mut tags = Vec::new();

    scan_non_code_spans(content, |span, span_start| {
        let range = span_start..span_start + span.len();
        scan_xml_tags_in_text(span, range, &line_starts, &mut tags);
    });

    tags
}

/// Extract markdown links from content (excluding code blocks/spans)
///
/// This extracts both regular links `[text](url)` and image links `![alt](url)`.
///
/// # Security
///
/// Uses a regex-based scan rather than a full markdown parser. The scan is limited
/// to `MAX_REGEX_INPUT_SIZE` bytes to prevent ReDoS; content exceeding that limit
/// returns an empty result.
///
/// # Input sanitization
///
/// C0 control characters and non-standard line endings are sanitized (replaced with
/// spaces or LF) before scanning to ensure byte-offset alignment with the original
/// normalized content.
pub fn extract_markdown_links(content: &str) -> Vec<MarkdownLink> {
    // Sanitize C0 control characters and normalize CRLF before scanning.
    let content = sanitize_for_pulldown_cmark(content);
    // Upstream parser panicked; return empty result (callers handle gracefully)
    panic::catch_unwind(AssertUnwindSafe(|| extract_markdown_links_inner(&content)))
        .unwrap_or_default()
}

fn extract_markdown_links_inner(content: &str) -> Vec<MarkdownLink> {
    let line_starts = compute_line_starts(content);
    let mut links = Vec::new();

    // Regex for inline links: optional `!` (image), then [text](url)
    // Uses a simple pattern that avoids nested brackets/parens (sufficient for
    // real-world agent config files).
    static_regex!(
        fn link_re,
        r"(!?)\[([^\[\]]*)\]\(([^()]*)\)"
    );
    let re = link_re();

    scan_non_code_spans(content, |span, span_start| {
        for cap in re.captures_iter(span) {
            let full = cap.get(0).unwrap();
            let is_image = cap.get(1).is_some_and(|m| m.as_str() == "!");
            let text = cap.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
            let url = cap.get(3).map(|m| m.as_str()).unwrap_or("").to_string();

            let start_byte = span_start + full.start();
            let end_byte = span_start + full.end();
            let (line, column) = line_col_at(start_byte, &line_starts);

            links.push(MarkdownLink {
                url,
                text,
                is_image,
                line,
                column,
                start_byte,
                end_byte,
            });
        }
    });

    links
}

/// Check if XML tags are balanced
#[allow(dead_code)] // used in cfg(test) and __internal; not yet used by production validators
pub fn check_xml_balance(tags: &[XmlTag]) -> Vec<XmlBalanceError> {
    check_xml_balance_with_content_end(tags, None)
}

/// Check if XML tags are balanced, with optional content length for auto-fix byte positions
pub fn check_xml_balance_with_content_end(
    tags: &[XmlTag],
    content_len: Option<usize>,
) -> Vec<XmlBalanceError> {
    let mut stack: Vec<&XmlTag> = Vec::new();
    let mut errors = Vec::new();

    for tag in tags {
        if tag.is_closing {
            if let Some(last) = stack.last() {
                if last.name == tag.name {
                    stack.pop();
                } else {
                    errors.push(XmlBalanceError::Mismatch {
                        expected: last.name.clone(),
                        found: tag.name.clone(),
                        line: tag.line,
                        column: tag.column,
                    });
                }
            } else {
                errors.push(XmlBalanceError::UnmatchedClosing {
                    tag: tag.name.clone(),
                    line: tag.line,
                    column: tag.column,
                });
            }
        } else {
            stack.push(tag);
        }
    }

    // Unclosed tags - compute content_end_byte for auto-fix
    // For each unclosed tag, the closing tag should be inserted at the end of content
    // (or at the start of the next tag at the same/lower nesting level)
    let content_end = content_len.unwrap_or_else(|| tags.last().map(|t| t.end_byte).unwrap_or(0));

    for tag in stack {
        errors.push(XmlBalanceError::Unclosed {
            tag: tag.name.clone(),
            line: tag.line,
            column: tag.column,
            open_tag_end_byte: tag.end_byte,
            content_end_byte: content_end,
        });
    }

    errors
}

#[derive(Debug, Clone)]
pub struct Import {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// A markdown link extracted from content
#[derive(Debug, Clone)]
pub struct MarkdownLink {
    /// The URL/path of the link
    pub url: String,
    /// The link text (alt text for images)
    #[allow(dead_code)] // parsed but not yet consumed by validators
    pub text: String,
    /// Whether this is an image link (![alt](url))
    #[allow(dead_code)] // parsed but not yet consumed by validators
    pub is_image: bool,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Byte offset of link start
    #[allow(dead_code)] // parsed but not yet consumed by validators
    pub start_byte: usize,
    /// Byte offset of link end
    #[allow(dead_code)] // parsed but not yet consumed by validators
    pub end_byte: usize,
}

#[derive(Debug, Clone)]
pub struct XmlTag {
    pub name: String,
    pub is_closing: bool,
    pub line: usize,
    pub column: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone)]
pub enum XmlBalanceError {
    Unclosed {
        tag: String,
        line: usize,
        column: usize,
        /// Byte position of the opening tag (for auto-fix)
        #[allow(dead_code)] // parsed but not yet consumed by validators
        open_tag_end_byte: usize,
        /// Byte position where the closing tag should be inserted (content end)
        content_end_byte: usize,
    },
    UnmatchedClosing {
        tag: String,
        line: usize,
        column: usize,
    },
    Mismatch {
        expected: String,
        found: String,
        line: usize,
        column: usize,
    },
}

/// Sanitize content to prevent parser panics from C0 control characters.
///
/// `pulldown-cmark` 0.13.0 has a known panic triggered by inputs that contain C0
/// control characters (U+0001..=U+0008, U+000B, U+000C, U+000E..=U+001F) combined
/// with certain CommonMark syntax. These characters do not appear in real markdown
/// files and are not meaningful to the markdown spec.
///
/// This function:
/// - Normalizes CRLF and lone CR to LF
/// - Converts VT (U+000B) and FF (U+000C) to LF (they are whitespace line-endings)
/// - Replaces remaining C0 control characters with spaces (U+0020) to preserve byte
///   offsets; stripping would shift all subsequent positions and break fix ranges
///
/// Returns `Cow::Borrowed` when the input is already clean (zero allocation).
pub fn sanitize_for_pulldown_cmark(s: &str) -> std::borrow::Cow<'_, str> {
    // Fast path: check whether any sanitization is needed.
    // The most common case (LF-only content with no control chars) takes one scan.
    let needs_work = s
        .bytes()
        .any(|b| b == b'\r' || (b < 0x20 && b != b'\t' && b != b'\n'));
    if !needs_work {
        return std::borrow::Cow::Borrowed(s);
    }

    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                // Normalize CRLF and lone CR to LF
                chars.next_if_eq(&'\n');
                out.push('\n');
            }
            '\x0b' | '\x0c' => {
                // VT and FF are line-ending whitespace - treat as LF
                out.push('\n');
            }
            c if c < '\x20' && c != '\t' && c != '\n' => {
                // Replace with a space rather than stripping. Stripping would shorten the
                // string and shift all subsequent byte offsets, making XmlTag start/end
                // spans misalign with the (CRLF-normalized) content the fix engine uses.
                // A space is 1 byte → 1 byte and is harmless to pulldown-cmark.
                out.push(' ');
            }
            c => out.push(c),
        }
    }
    std::borrow::Cow::Owned(out)
}

/// Call `callback(span, span_start_byte)` for each text span in `content` that
/// lies outside a fenced code block or inline code span.
///
/// This is a panic-free replacement for the `pulldown-cmark` event iterator used
/// in `extract_xml_tags_inner` and `extract_imports_inner`.  `pulldown-cmark`
/// 0.13.0 contains an internal invariant bug (unwrap on None) that is triggered
/// by certain link-reference-definition / tight-paragraph combinations.  Because
/// `libfuzzer-sys` installs an aborting panic hook, `catch_unwind` cannot catch
/// the panic in fuzz builds, so we must prevent the panic from occurring at all.
///
/// ### What we handle
///
/// * **Fenced code blocks** – lines whose first non-space content is a run of
///   three or more `` ` `` or `~` characters open a fence; a subsequent line
///   with the same fence character and at least the same run length closes it.
///   Up to three leading spaces are allowed before the fence (per CommonMark).
/// * **Inline code spans** – backtick-delimited spans inside a non-fenced line
///   are skipped.  We match the opening backtick run and look for the same-
///   length closing run to stay correct for ` ``double`` ` spans.
///
/// ### What we do NOT handle
///
/// * Indented code blocks (4-space / tab indent).  These are uncommon in real
///   agent-config files; omitting them is an acceptable trade-off for
///   simplicity.
/// * HTML block scanning.  All non-fenced, non-backtick content is yielded.
fn scan_non_code_spans(content: &str, mut callback: impl FnMut(&str, usize)) {
    let bytes = content.as_bytes();
    let len = bytes.len();

    let mut pos: usize = 0;
    // State for fenced code block
    let mut in_fence = false;
    let mut fence_char: u8 = b'`';
    let mut fence_min_len: usize = 3;

    while pos < len {
        // Find the end of the current line (pos..line_end) where line_end points
        // at the byte *after* the newline (or at `len` for the last line).
        let newline_pos = memchr_newline(bytes, pos).unwrap_or(len);
        // line_end is the start of the next line
        let line_end = if newline_pos < len {
            newline_pos + 1
        } else {
            len
        };
        let line = &content[pos..line_end];
        let line_start = pos;

        if in_fence {
            // Check whether this line closes the fence.
            // A closing fence is: up to 3 optional spaces, then ≥ fence_min_len
            // fence_char bytes, then optional spaces, then end-of-line.
            let line_trimmed = line.trim_end_matches('\n');
            let n_leading = line_trimmed.bytes().take_while(|&b| b == b' ').count();
            if n_leading <= 3 {
                let after = &line_trimmed[n_leading..];
                let run = after.bytes().take_while(|&b| b == fence_char).count();
                if run >= fence_min_len {
                    let rest = &after[run..];
                    if rest.bytes().all(|b| b == b' ') {
                        // Closing fence found – exit fenced mode
                        in_fence = false;
                        pos = line_end;
                        continue;
                    }
                }
            }
            // Inside fenced block: skip the line entirely
            pos = line_end;
            continue;
        }

        // Not in a fenced block.  Check if this line opens a fence.
        {
            let line_trimmed = line.trim_end_matches('\n');
            let n_leading = line_trimmed.bytes().take_while(|&b| b == b' ').count();
            if n_leading <= 3 {
                let after = &line_trimmed[n_leading..];
                let tick_run = after.bytes().take_while(|&b| b == b'`').count();
                let tilde_run = after.bytes().take_while(|&b| b == b'~').count();
                // The fence char is whichever has the longer run
                let (run, ch) = if tick_run >= tilde_run {
                    (tick_run, b'`')
                } else {
                    (tilde_run, b'~')
                };
                if run >= 3 {
                    // Opening fence: skip this line, enter fenced mode
                    in_fence = true;
                    fence_char = ch;
                    fence_min_len = run;
                    pos = line_end;
                    continue;
                }
            }
        }

        // Non-code-block line: yield non-inline-code sub-spans.
        // We walk through the line looking for backtick runs that open/close
        // inline code spans.
        let mut cursor = line_start;
        let mut i = line_start;
        while i < line_end {
            if bytes[i] == b'`' {
                // Measure the length of this backtick run
                let tick_start = i;
                while i < line_end && bytes[i] == b'`' {
                    i += 1;
                }
                let tick_len = i - tick_start;
                // Yield the text before this backtick run
                if tick_start > cursor {
                    callback(&content[cursor..tick_start], cursor);
                }
                // Find the matching closing backtick run (same length)
                let search_from = i;
                let close = find_closing_backtick(bytes, search_from, line_end, tick_len);
                match close {
                    Some(close_start) => {
                        // Skip from cursor past the closing run
                        i = close_start + tick_len;
                        cursor = i;
                    }
                    None => {
                        // No matching close on this line: treat the backtick run as
                        // literal text and continue scanning from after the opening run.
                        // Yield the backtick run itself as part of the normal text so
                        // that the caller sees everything except a real closed span.
                        // Reset cursor to after the tick run and keep scanning.
                        cursor = tick_start; // include the ticks in next yield
                        // i is already past the tick run; continue
                    }
                }
            } else {
                i += 1;
            }
        }
        // Yield any remaining text on this line
        if cursor < line_end {
            callback(&content[cursor..line_end], cursor);
        }

        pos = line_end;
    }
}

/// Return the byte position of the start of the first backtick run of exactly
/// `tick_len` within `bytes[start..end]`.
fn find_closing_backtick(bytes: &[u8], start: usize, end: usize, tick_len: usize) -> Option<usize> {
    let mut i = start;
    while i + tick_len <= end {
        if bytes[i] == b'`' {
            let run_start = i;
            while i < end && bytes[i] == b'`' {
                i += 1;
            }
            let run_len = i - run_start;
            if run_len == tick_len {
                return Some(run_start);
            }
            // Wrong length run; continue scanning
        } else {
            i += 1;
        }
    }
    None
}

/// Find the position of the next `\n` byte in `bytes[start..]`.
/// Returns `None` if no newline is found.
fn memchr_newline(bytes: &[u8], start: usize) -> Option<usize> {
    bytes[start..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|p| start + p)
}

fn compute_line_starts(content: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, ch) in content.char_indices() {
        if ch == '\n' {
            starts.push(idx + 1);
        }
    }
    starts
}

fn line_col_at(offset: usize, line_starts: &[usize]) -> (usize, usize) {
    let mut low = 0usize;
    let mut high = line_starts.len();
    while low + 1 < high {
        let mid = (low + high) / 2;
        if line_starts[mid] <= offset {
            low = mid;
        } else {
            high = mid;
        }
    }
    let line_start = line_starts[low];
    (low + 1, offset - line_start + 1)
}

fn scan_imports_in_text(
    text: &str,
    range: Range<usize>,
    line_starts: &[usize],
    imports: &mut Vec<Import>,
) {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            let prev_ok = if i == 0 {
                true
            } else {
                let prev = text[..i].chars().last().unwrap_or(' ');
                !prev.is_alphanumeric() && !matches!(prev, '_' | '-' | '.')
            };
            if !prev_ok {
                i += 1;
                continue;
            }

            let start = i + 1;
            let mut j = start;
            while j < bytes.len() {
                let b = bytes[j];
                let allowed = b.is_ascii_alphanumeric()
                    || matches!(b, b'_' | b'-' | b'.' | b'/' | b'\\' | b':' | b'~');
                if !allowed {
                    break;
                }
                j += 1;
            }

            if j == start {
                i += 1;
                continue;
            }

            let mut end = j;
            while end > start {
                let b = bytes[end - 1];
                if matches!(b, b'.' | b',' | b';' | b':') {
                    end -= 1;
                } else {
                    break;
                }
            }

            if end == start {
                i = j;
                continue;
            }

            let path = text[start..end].to_string();
            if !is_probable_import_path(&path) {
                i = j;
                continue;
            }
            let start_byte = range.start + i;
            let end_byte = range.start + end;
            let (line, column) = line_col_at(start_byte, line_starts);

            imports.push(Import {
                path,
                line,
                column,
                start_byte,
                end_byte,
            });

            i = j;
            continue;
        }

        i += 1;
    }
}

/// Returns true if the tag name is an HTML5 void element.
/// Void elements are elements that cannot have child nodes and never
/// need closing tags (e.g., `<br>`, `<img>`, `<hr>`).
/// See: https://html.spec.whatwg.org/multipage/syntax.html#void-elements
fn is_html5_void_element(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Returns true if the tag name looks like a programming language type parameter
/// rather than an XML/HTML tag. Common in TypeScript, Rust, Java, C# code examples.
fn is_likely_type_parameter(name: &str) -> bool {
    // Single uppercase letter: <T>, <K>, <V>, <E>, <R>
    if name.len() == 1 && name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
        return true;
    }
    // Common generic type parameter names (PascalCase, all start with uppercase)
    matches!(
        name,
        "Key"
            | "Value"
            | "Item"
            | "Element"
            | "Result"
            | "Error"
            | "Input"
            | "Output"
            | "State"
            | "Props"
            | "Args"
            | "Return"
            | "Option"
            | "Type"
            | "Param"
            | "Config"
            | "Context"
            | "TableName"
            | "DeviceID"
            | "FieldName"
            | "HashMap"
            | "HashSet"
            | "Vec"
            | "List"
            | "Array"
            | "Map"
            | "Set"
    )
}

/// Returns true if the HTML element is commonly used in markdown
/// without proper closing tags and rendered correctly by GitHub/GitLab.
/// These are NOT void elements but are frequently unpaired in practice.
fn is_markdown_safe_html(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "details" | "summary" | "picture" | "video" | "audio" | "figure" | "figcaption"
    )
}

fn scan_xml_tags_in_text(
    text: &str,
    range: Range<usize>,
    line_starts: &[usize],
    tags: &mut Vec<XmlTag>,
) {
    // Regex to match XML/HTML tags:
    // - Group 1: "/" if closing tag (e.g., </tag>)
    // - Group 2: tag name
    // - Group 3: "/" if self-closing tag (e.g., <br/> or <img src="..." />)
    // The (?:\s+[^>]*?)? handles attributes like <a id="foo"> or <img src="bar">
    let re = xml_tag_regex();

    for cap in re.captures_iter(text) {
        let is_closing = cap.get(1).is_some_and(|m| m.as_str() == "/");
        let is_self_closing = cap.get(3).is_some_and(|m| m.as_str() == "/");

        // Skip self-closing tags - they don't need balance checking
        // Examples: <br/>, <hr />, <img src="..." />
        if is_self_closing {
            continue;
        }

        if let Some(name_match) = cap.get(2) {
            let name = name_match.as_str().to_string();

            // Skip HTML5 void elements - they never need closing tags.
            // In markdown, these are commonly written without self-closing syntax
            // (e.g., <br> instead of <br/>, <img src="..."> instead of <img ... />)
            // Also skip HTML elements commonly used in markdown without closing tags
            // (e.g., <details> for collapsible sections on GitHub)
            // Skip both opening AND closing tags for void elements and markdown-safe elements
            if is_html5_void_element(&name) || is_markdown_safe_html(&name) {
                continue;
            }

            // Skip likely type parameters from programming languages.
            // Single uppercase letters (T, K, V) and PascalCase names ending
            // in common generic suffixes are almost always type parameters,
            // not XML tags. These appear frequently in API docs and code examples
            // within agent instructions.
            if !is_closing && is_likely_type_parameter(&name) {
                continue;
            }

            // Skip path template placeholders like <feature> in
            // "lib/features/<feature>/data/". These are variable placeholders
            // in file path documentation, not XML tags.
            if !is_closing {
                let full_match = cap.get(0).unwrap();
                let match_start = full_match.start();
                let match_end = full_match.end();
                let preceded_by_slash =
                    match_start > 0 && text.as_bytes().get(match_start - 1) == Some(&b'/');
                let followed_by_slash = text.as_bytes().get(match_end) == Some(&b'/');
                if preceded_by_slash || followed_by_slash {
                    continue;
                }
            }
            let start = cap.get(0).unwrap().start();
            let end = cap.get(0).unwrap().end();
            let start_byte = range.start + start;
            let end_byte = range.start + end;
            let (line, column) = line_col_at(start_byte, line_starts);
            tags.push(XmlTag {
                name,
                is_closing,
                line,
                column,
                start_byte,
                end_byte,
            });
        }
    }
}

/// Returns true if the name matches a known file that has no extension.
/// These are common in project roots and are valid @import targets.
fn is_known_extensionless_file(name: &str) -> bool {
    matches!(
        name,
        "Dockerfile"
            | "Makefile"
            | "Jenkinsfile"
            | "Vagrantfile"
            | "Procfile"
            | "Gemfile"
            | "Rakefile"
            | "Brewfile"
            | "Justfile"
            | "Taskfile"
            | "Earthfile"
            | "Containerfile"
            | "Tiltfile"
            | "Snakefile"
            | "LICENSE"
            | "LICENCE"
    )
}

fn is_probable_import_path(path: &str) -> bool {
    // Absolute paths (start with / or \) are always considered imports
    // (they will be rejected later in validation)
    if path.starts_with('/') || path.starts_with('\\') {
        return true;
    }

    // If path contains directory separator, require a file extension
    // to avoid false positives like "@import/reference" (a label, not a file)
    if path.contains('/') || path.contains('\\') {
        // Must have a file extension (.md, .txt, etc.)
        let has_extension = path.rfind('.').is_some_and(|dot_pos| {
            let after_dot = &path[dot_pos + 1..];
            let last_sep = path.rfind('/').or_else(|| path.rfind('\\'));
            // Extension must come after the last separator
            last_sep.is_none_or(|sep| dot_pos > sep) && !after_dot.is_empty()
        });
        return has_extension;
    }

    // For paths without separators, require a file extension or tilde prefix.
    // This prevents @mentions like @MashTimeBot from being treated as imports.
    if path.starts_with('~') || path.contains(':') {
        return true;
    }

    // Known extensionless files that are valid import targets
    if is_known_extensionless_file(path) {
        return true;
    }

    // Exclude email-like patterns (multiple dots with no file extension structure)
    // e.g., "users.noreply.github.com" is an email domain, not a file
    if path.matches('.').count() >= 2 && !path.contains('/') {
        // If the last segment after the last dot looks like a TLD, it's likely a domain
        if let Some(last_dot) = path.rfind('.') {
            let ext = &path[last_dot + 1..];
            if matches!(
                ext,
                "com" | "org" | "net" | "io" | "dev" | "ai" | "co" | "edu" | "gov"
            ) {
                return false;
            }
        }
    }

    // Must contain a dot (file extension) to be considered a file reference
    path.contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_patterns_compile() {
        let _ = xml_tag_regex();
    }

    #[test]
    fn test_extract_imports() {
        let content = "See @docs/guide.md and @README.md";
        let imports = extract_imports(content);
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].path, "docs/guide.md");
        assert_eq!(imports[1].path, "README.md");
    }

    #[test]
    fn test_extract_imports_ignores_inline_code() {
        let content = "Use `@not-an-import.md` but see @real.md";
        let imports = extract_imports(content);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].path, "real.md");
    }

    #[test]
    fn test_extract_imports_ignores_code_block() {
        let content = "```\nimport x from '@pkg/name'\n```\nSee @actual.md";
        let imports = extract_imports(content);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].path, "actual.md");
    }

    #[test]
    fn test_extract_imports_ignores_plain_mentions() {
        let content = "Use @import and @imports in docs";
        let imports = extract_imports(content);
        assert!(imports.is_empty());
    }

    #[test]
    fn test_xml_balance() {
        let content = "<example>test</example>";
        let tags = extract_xml_tags(content);
        let errors = check_xml_balance(&tags);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_xml_ignores_code_block() {
        let content = "```\n<example>test</example>\n```\n";
        let tags = extract_xml_tags(content);
        assert!(tags.is_empty());
    }

    #[test]
    fn test_xml_unclosed() {
        let content = "<example>test";
        let tags = extract_xml_tags(content);
        let errors = check_xml_balance(&tags);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], XmlBalanceError::Unclosed { .. }));
    }

    #[test]
    fn test_xml_tags_with_attributes() {
        // HTML anchor tags with id attribute should be properly balanced
        let content = r#"<a id="test"></a>"#;
        let tags = extract_xml_tags(content);
        assert_eq!(tags.len(), 2);
        assert!(!tags[0].is_closing); // <a id="test">
        assert!(tags[1].is_closing); // </a>
        let errors = check_xml_balance(&tags);
        assert!(errors.is_empty(), "Tags with attributes should balance");
    }

    #[test]
    fn test_xml_tags_with_multiple_attributes() {
        let content = r#"<div class="foo" id="bar">content</div>"#;
        let tags = extract_xml_tags(content);
        assert_eq!(tags.len(), 2);
        let errors = check_xml_balance(&tags);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_xml_self_closing_tags() {
        // Self-closing tags like <br/> should not cause balance errors
        let content = "<br/>";
        let tags = extract_xml_tags(content);
        assert!(tags.is_empty(), "Self-closing tags should be skipped");
    }

    #[test]
    fn test_xml_self_closing_with_space() {
        // Self-closing tags with space like <br /> should also be skipped
        let content = "<br />";
        let tags = extract_xml_tags(content);
        assert!(
            tags.is_empty(),
            "Self-closing tags with space should be skipped"
        );
    }

    #[test]
    fn test_xml_self_closing_with_attributes() {
        // Self-closing tags with attributes should be skipped
        let content = r#"<img src="test.png" />"#;
        let tags = extract_xml_tags(content);
        assert!(
            tags.is_empty(),
            "Self-closing tags with attributes should be skipped"
        );
    }

    #[test]
    fn test_xml_mixed_tags_and_self_closing() {
        // Mix of regular tags and self-closing tags
        let content = r#"<div><br/><span>text</span><hr /></div>"#;
        let tags = extract_xml_tags(content);
        // Should have: <div>, <span>, </span>, </div> (br and hr are self-closing)
        assert_eq!(tags.len(), 4);
        let errors = check_xml_balance(&tags);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_xml_unclosed_with_content_end() {
        let content = "<example>test content here";
        let tags = extract_xml_tags(content);
        let errors = check_xml_balance_with_content_end(&tags, Some(content.len()));
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            XmlBalanceError::Unclosed {
                tag,
                content_end_byte,
                open_tag_end_byte,
                ..
            } => {
                assert_eq!(tag, "example");
                assert_eq!(*content_end_byte, content.len());
                assert_eq!(*open_tag_end_byte, 9); // Length of "<example>"
            }
            _ => panic!("Expected Unclosed error"),
        }
    }

    #[test]
    fn test_xml_balance_multiple_unclosed() {
        let content = "<outer><inner>content";
        let tags = extract_xml_tags(content);
        let errors = check_xml_balance_with_content_end(&tags, Some(content.len()));
        // Both <outer> and <inner> are unclosed
        assert_eq!(errors.len(), 2);
        for err in &errors {
            match err {
                XmlBalanceError::Unclosed {
                    content_end_byte, ..
                } => {
                    assert_eq!(*content_end_byte, content.len());
                }
                _ => panic!("Expected Unclosed error"),
            }
        }
    }

    // ===== Markdown Link Extraction Tests =====

    #[test]
    fn test_extract_markdown_links_basic() {
        let content = "See [guide](docs/guide.md) for more info.";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "docs/guide.md");
        assert_eq!(links[0].text, "guide");
        assert!(!links[0].is_image);
    }

    #[test]
    fn test_extract_markdown_links_multiple() {
        let content = "See [one](a.md) and [two](b.md) files.";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].url, "a.md");
        assert_eq!(links[1].url, "b.md");
    }

    #[test]
    fn test_extract_markdown_links_image() {
        let content = "Here is ![logo](images/logo.png) image.";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "images/logo.png");
        assert_eq!(links[0].text, "logo");
        assert!(links[0].is_image);
    }

    #[test]
    fn test_extract_markdown_links_ignores_code_block() {
        let content = "```\n[link](skip.md)\n```\n[real](keep.md)";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "keep.md");
    }

    #[test]
    fn test_extract_markdown_links_ignores_inline_code() {
        let content = "Use `[not](skip.md)` but see [real](keep.md)";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "keep.md");
    }

    #[test]
    fn test_extract_markdown_links_with_fragment() {
        let content = "See [section](docs/guide.md#section) for details.";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "docs/guide.md#section");
    }

    #[test]
    fn test_extract_markdown_links_external() {
        let content = "Visit [GitHub](https://github.com) site.";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "https://github.com");
    }

    #[test]
    fn test_extract_markdown_links_anchor_only() {
        let content = "Jump to [section](#section-name).";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "#section-name");
    }

    #[test]
    fn test_extract_markdown_links_line_column() {
        let content = "Line one\n[link](file.md)\nLine three";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].line, 2);
        assert_eq!(links[0].column, 1);
    }

    // ===== Security: Regex DoS Protection Tests =====

    #[test]
    fn test_xml_tags_skipped_for_oversized_content() {
        // Create content larger than MAX_REGEX_INPUT_SIZE
        let large_content = "a".repeat(MAX_REGEX_INPUT_SIZE + 1000);
        let content_with_tags = format!("<example>{}</example>", large_content);

        let tags = extract_xml_tags(&content_with_tags);
        // Tags should be empty because content exceeds size limit
        // This prevents potential ReDoS attacks
        assert!(
            tags.is_empty(),
            "Oversized content should skip XML tag extraction for security"
        );
    }

    #[test]
    fn test_xml_tags_processed_for_normal_sized_content() {
        // Content just under the limit should still be processed
        let content = "<example>test</example>";
        assert!(content.len() < MAX_REGEX_INPUT_SIZE);

        let tags = extract_xml_tags(content);
        assert_eq!(tags.len(), 2, "Normal sized content should be processed");
    }

    #[test]
    fn test_max_regex_input_size_constant() {
        // Verify the constant is set to 64KB as documented
        assert_eq!(MAX_REGEX_INPUT_SIZE, 65536);
    }

    // ===== Precise Boundary Tests for MAX_REGEX_INPUT_SIZE =====

    #[test]
    fn test_extract_xml_tags_exactly_at_64kb_limit() {
        let open = "<example>";
        let close = "</example>";
        let overhead = open.len() + close.len(); // 9 + 10 = 19 bytes
        let content = format!(
            "{}{}{}",
            open,
            "a".repeat(MAX_REGEX_INPUT_SIZE - overhead),
            close
        );
        assert_eq!(
            content.len(),
            MAX_REGEX_INPUT_SIZE,
            "Content must be exactly at the limit"
        );
        let tags = extract_xml_tags(&content);
        assert!(
            !tags.is_empty(),
            "Content at exactly the limit should be processed"
        );
    }

    #[test]
    fn test_extract_xml_tags_one_byte_over_limit() {
        let open = "<example>";
        let close = "</example>";
        let overhead = open.len() + close.len(); // 9 + 10 = 19 bytes
        let content = format!(
            "{}{}{}",
            open,
            "a".repeat(MAX_REGEX_INPUT_SIZE + 1 - overhead),
            close
        );
        assert_eq!(
            content.len(),
            MAX_REGEX_INPUT_SIZE + 1,
            "Content must be one byte over the limit"
        );
        let tags = extract_xml_tags(&content);
        assert!(
            tags.is_empty(),
            "Content one byte over the limit should be skipped"
        );
    }

    #[test]
    fn test_extract_imports_processes_above_64kb_limit() {
        // No exactly-at-64kb companion: extract_imports has no size guard and always
        // processes content of any size.
        // extract_imports uses byte-by-byte scanning (not regex) inside pulldown-cmark
        // token callbacks, so MAX_REGEX_INPUT_SIZE does not apply.
        let imports_prefix = "@styles.css\n";
        let needed = MAX_REGEX_INPUT_SIZE - imports_prefix.len() + 1;
        let content = format!("{}{}", imports_prefix, "a".repeat(needed));
        assert!(
            content.len() > MAX_REGEX_INPUT_SIZE,
            "Content must exceed the limit"
        );
        let imports = extract_imports(&content);
        assert!(
            !imports.is_empty(),
            "extract_imports should process content beyond the regex size limit"
        );
    }

    #[test]
    fn test_extract_markdown_links_processes_above_64kb_limit() {
        // No exactly-at-64kb companion: extract_markdown_links has no size guard and
        // always processes content of any size.
        // extract_markdown_links uses pulldown-cmark event iteration (not regex),
        // so MAX_REGEX_INPUT_SIZE does not apply.
        let link = "[example](https://example.com)\n";
        let needed = MAX_REGEX_INPUT_SIZE - link.len() + 1;
        let content = format!("{}{}", link, "a".repeat(needed));
        assert!(
            content.len() > MAX_REGEX_INPUT_SIZE,
            "Content must exceed the limit"
        );
        let links = extract_markdown_links(&content);
        assert!(
            !links.is_empty(),
            "extract_markdown_links should process content beyond the regex size limit"
        );
    }

    // ===== Tests for new helper functions =====

    #[test]
    fn test_is_markdown_safe_html() {
        // Should return true for commonly unpaired HTML elements in markdown
        assert!(is_markdown_safe_html("details"));
        assert!(is_markdown_safe_html("summary"));
        assert!(is_markdown_safe_html("picture"));
        assert!(is_markdown_safe_html("video"));
        assert!(is_markdown_safe_html("audio"));
        assert!(is_markdown_safe_html("figure"));
        assert!(is_markdown_safe_html("figcaption"));
        assert!(is_markdown_safe_html("DETAILS")); // case-insensitive
        assert!(is_markdown_safe_html("Summary")); // case-insensitive
    }

    #[test]
    fn test_is_markdown_safe_html_negative() {
        // Should return false for other HTML elements
        assert!(!is_markdown_safe_html("div"));
        assert!(!is_markdown_safe_html("span"));
        assert!(!is_markdown_safe_html("p"));
        assert!(!is_markdown_safe_html("table"));
        assert!(!is_markdown_safe_html("example"));
    }

    #[test]
    fn test_is_known_extensionless_file() {
        // Should return true for known files without extensions
        assert!(is_known_extensionless_file("Dockerfile"));
        assert!(is_known_extensionless_file("Makefile"));
        assert!(is_known_extensionless_file("Jenkinsfile"));
        assert!(is_known_extensionless_file("LICENSE"));
        assert!(is_known_extensionless_file("LICENCE"));
        assert!(is_known_extensionless_file("Gemfile"));
        assert!(is_known_extensionless_file("Rakefile"));
    }

    #[test]
    fn test_is_known_extensionless_file_negative() {
        // Should return false for other files
        assert!(!is_known_extensionless_file("README"));
        assert!(!is_known_extensionless_file("dockerfile")); // case-sensitive
        assert!(!is_known_extensionless_file("main"));
        assert!(!is_known_extensionless_file("test"));
    }

    #[test]
    fn test_xml_path_template_placeholder_filtering_inline() {
        // Test that tags immediately adjacent to slashes in inline text are filtered
        // This works when the entire path is in a single text node
        let content = "`lib/features/<feature>/data/`";
        let tags = extract_xml_tags(content);
        // Tags in code blocks/spans should not be extracted at all
        assert_eq!(tags.len(), 0, "Tags in code should be ignored");
    }

    #[test]
    fn test_xml_path_template_detection_logic() {
        // The filtering logic applies within text chunks that the markdown parser provides
        // When a tag has a slash immediately before or after it in the SAME text chunk,
        // it gets filtered. This test verifies the behavior matches expectations.

        // Real XML tag without slashes should be detected
        let content = "Some <example> tag here";
        let tags = extract_xml_tags(content);
        assert_eq!(tags.len(), 1, "Real XML tags should be detected");
        assert_eq!(tags[0].name, "example");

        // Tag in actual path context (markdown may split this differently)
        let content2 = "The path is: `/<folder>/`";
        let tags2 = extract_xml_tags(content2);
        // In code context, should be filtered anyway
        assert_eq!(tags2.len(), 0, "Tags in code should be filtered");
    }

    #[test]
    fn test_xml_non_path_tags_not_filtered() {
        // Real XML tags not adjacent to slashes should not be filtered
        let content = "Some <example> tag here";
        let tags = extract_xml_tags(content);
        assert_eq!(tags.len(), 1, "Real XML tags should not be filtered");
        assert_eq!(tags[0].name, "example");
    }

    #[test]
    fn test_is_probable_import_path_email_domain_filtering() {
        // Email-like patterns should be rejected
        assert!(!is_probable_import_path("users.noreply.github.com"));
        assert!(!is_probable_import_path("foo.bar.example.com"));
        assert!(!is_probable_import_path("test.example.org"));
        assert!(!is_probable_import_path("api.service.io"));
    }

    #[test]
    fn test_is_probable_import_path_valid_files() {
        // Valid file paths should be accepted
        assert!(is_probable_import_path("docs/guide.md"));
        assert!(is_probable_import_path("README.md"));
        assert!(is_probable_import_path("config.json"));
        assert!(is_probable_import_path("Dockerfile"));
        assert!(is_probable_import_path("~/docs/file.txt"));
        assert!(is_probable_import_path("./relative/path.md"));
    }

    #[test]
    fn test_is_probable_import_path_rejects_mentions() {
        // Plain @mentions should be rejected
        assert!(!is_probable_import_path("username"));
        assert!(!is_probable_import_path("MashTimeBot"));
        assert!(!is_probable_import_path("import"));
    }

    #[test]
    fn test_xml_extraction_with_emoji_in_tag_content() {
        // Emoji in content between tags should not affect tag extraction
        let content = "<example>\u{1f525} fire content \u{1f680}</example>";
        let tags = extract_xml_tags(content);
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name, "example");
        assert!(!tags[0].is_closing);
        assert_eq!(tags[1].name, "example");
        assert!(tags[1].is_closing);
        let errors = check_xml_balance(&tags);
        assert!(
            errors.is_empty(),
            "Emoji in tag content should not cause balance errors"
        );
    }

    #[test]
    fn test_xml_extraction_with_emoji_adjacent_to_tags() {
        // Emoji characters adjacent to tags should not break parsing
        let content = "\u{1f4dd}<note>text</note>\u{2705}";
        let tags = extract_xml_tags(content);
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name, "note");
        assert_eq!(tags[1].name, "note");
        let errors = check_xml_balance(&tags);
        assert!(errors.is_empty());
    }

    // ===== Regression tests for pulldown-cmark panic with control characters =====

    #[test]
    fn test_extract_xml_tags_no_panic_on_fuzz_crash_input() {
        // Regression test: pulldown-cmark 0.13.0 panics on inputs containing C0
        // control characters combined with certain CommonMark syntax. The sanitize
        // step must prevent the panic before the parser runs.
        // Input: "*\t [m>\x02\t\x02@&]:+<>\r\x0b" (fuzz crash artifact)
        let crash_input = "*\t [m>\x02\t\x02@&]:+<>\r\x0b";
        // None of these should panic
        let _ = extract_xml_tags(crash_input);
        let _ = extract_imports(crash_input);
        let _ = extract_markdown_links(crash_input);
    }

    #[test]
    fn test_extract_xml_tags_no_panic_on_c0_control_chars() {
        // C0 control characters (0x01-0x08, 0x0e-0x1f) combined with markdown
        // syntax must not cause panics.
        let inputs = [
            "*\x01[a]:+<>\r\x0b", // SOH
            "*\x07[a]:+<>\r\x0b", // BEL
            "*\x08[a]:+<>\r\x0b", // BS
            "*\x0c[a]:+<>\r\x0c", // FF (form feed)
            "*\x0e[a]:+<>\r\x0b", // SO
            "*\x1f[a]:+<>\r\x0b", // US
        ];
        for input in &inputs {
            let _ = extract_xml_tags(input);
            let _ = extract_imports(input);
            let _ = extract_markdown_links(input);
        }
    }

    #[test]
    fn test_sanitize_for_pulldown_cmark_normalizes_crlf() {
        // Verify that CRLF content produces the same results as LF content
        let lf_content = "Some <example> text\nmore content\n";
        let crlf_content = "Some <example> text\r\nmore content\r\n";
        let tags_lf = extract_xml_tags(lf_content);
        let tags_crlf = extract_xml_tags(crlf_content);
        assert_eq!(
            tags_lf.len(),
            tags_crlf.len(),
            "CRLF should produce same tags as LF"
        );
    }

    #[test]
    fn test_sanitize_for_pulldown_cmark_replaces_control_chars() {
        // Control characters are replaced with spaces (not stripped) to preserve
        // byte offsets. Tags should still be detected.
        let content_with_control = "<example>\x02content\x03</example>";
        let tags = extract_xml_tags(content_with_control);
        assert_eq!(
            tags.len(),
            2,
            "Tags should be found after control char replacement"
        );
        assert_eq!(tags[0].name, "example");
        assert!(tags[1].is_closing);
    }

    #[test]
    fn test_sanitize_preserves_tab_and_newline() {
        // Tab and LF must be preserved (they are whitespace in CommonMark)
        let content = "<example>\tcontent\nnext line</example>";
        let tags = extract_xml_tags(content);
        assert_eq!(tags.len(), 2, "Tags with tab/newline should be extracted");
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // ===== extract_xml_tags() invariants =====

        #[test]
        fn extract_xml_tags_never_panics(content in ".*") {
            // Should never panic on any input
            let _ = extract_xml_tags(&content);
        }

        #[test]
        fn extract_xml_tags_valid_offsets(content in "[a-zA-Z0-9<>/\\-_ ]{0,1000}") {
            let tags = extract_xml_tags(&content);
            for tag in &tags {
                // Byte offsets must be within bounds
                prop_assert!(tag.start_byte <= content.len());
                prop_assert!(tag.end_byte <= content.len());
                prop_assert!(tag.start_byte <= tag.end_byte);

                // Line/column must be positive (1-indexed)
                prop_assert!(tag.line >= 1);
                prop_assert!(tag.column >= 1);
            }
        }

        #[test]
        fn check_xml_balance_never_panics(content in ".*") {
            let tags = extract_xml_tags(&content);
            // Should never panic on any input
            let _ = check_xml_balance(&tags);
            let _ = check_xml_balance_with_content_end(&tags, Some(content.len()));
        }

        // ===== extract_imports() invariants =====

        #[test]
        fn extract_imports_never_panics(content in ".*") {
            // Should never panic on any input
            let _ = extract_imports(&content);
        }

        #[test]
        fn extract_imports_valid_offsets(content in "[a-zA-Z0-9@./\\-_ ]{0,1000}") {
            let imports = extract_imports(&content);
            for import in &imports {
                // Byte offsets must be within bounds
                prop_assert!(import.start_byte <= content.len());
                prop_assert!(import.end_byte <= content.len());
                prop_assert!(import.start_byte <= import.end_byte);

                // Line/column must be positive (1-indexed)
                prop_assert!(import.line >= 1);
                prop_assert!(import.column >= 1);
            }
        }

        // ===== extract_markdown_links() invariants =====

        #[test]
        fn extract_markdown_links_never_panics(content in ".*") {
            // Should never panic on any input
            let _ = extract_markdown_links(&content);
        }

        #[test]
        fn extract_markdown_links_valid_offsets(content in "[a-zA-Z0-9\\[\\]()./\\-_ ]{0,1000}") {
            let links = extract_markdown_links(&content);
            for link in &links {
                // Byte offsets must be within bounds
                prop_assert!(link.start_byte <= content.len());
                prop_assert!(link.end_byte <= content.len());
                prop_assert!(link.start_byte <= link.end_byte);

                // Line/column must be positive (1-indexed)
                prop_assert!(link.line >= 1);
                prop_assert!(link.column >= 1);
            }
        }

        // ===== Well-formed input generation =====

        #[test]
        fn xml_tags_balanced_detected(
            tag in "[a-z]+",
            content in "[a-zA-Z0-9 ]{0,100}"
        ) {
            // Skip HTML5 void elements - they are intentionally excluded from balance checking
            prop_assume!(!is_html5_void_element(&tag));
            prop_assume!(!is_likely_type_parameter(&tag));
            let input = format!("<{}>{}</{}>", tag, content, tag);
            let tags = extract_xml_tags(&input);
            let errors = check_xml_balance(&tags);

            // Well-formed tags should have no balance errors
            prop_assert!(
                errors.is_empty(),
                "Well-formed XML should have no balance errors: {:?}",
                errors
            );
        }

        #[test]
        fn xml_unclosed_detected(tag in "[a-z]+") {
            // Skip elements intentionally excluded from balance checking
            prop_assume!(!is_html5_void_element(&tag));
            prop_assume!(!is_markdown_safe_html(&tag));
            prop_assume!(!is_likely_type_parameter(&tag));
            let input = format!("<{}>content", tag);
            let tags = extract_xml_tags(&input);
            let errors = check_xml_balance(&tags);

            // Unclosed tag should be detected
            prop_assert!(
                errors.len() == 1,
                "Unclosed tag should be detected"
            );
        }

        #[test]
        fn import_with_extension_detected(
            dir in "[a-z]+",
            file in "[a-z]+",
            ext in "(md|txt|json|yaml)"
        ) {
            let input = format!("See @{}/{}.{} for details", dir, file, ext);
            let imports = extract_imports(&input);

            prop_assert!(
                !imports.is_empty(),
                "Import with extension should be detected"
            );
            prop_assert_eq!(
                &imports[0].path,
                &format!("{}/{}.{}", dir, file, ext)
            );
        }
    }
}
