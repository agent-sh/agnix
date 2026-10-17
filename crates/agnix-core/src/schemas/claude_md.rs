//! CLAUDE.md and Claude Code memory-file helpers.
//!
//! The thresholds below that the Claude Code docs do not state are agnix
//! heuristics; each constant says which one it is.

use regex::Regex;
use std::collections::HashSet;

use crate::parsers::markdown::mask_code_and_html_comments;
use crate::regex_util::static_regex;

static_regex!(fn npm_run_pattern, r"npm\s+run\s+([a-zA-Z0-9_:-]+)");
static_regex!(fn emphasis_token_pattern, r"\b(IMPORTANT|CRITICAL|MUST|NEVER|ALWAYS|MANDATORY|DO NOT)\b");
static_regex!(fn rfc_keywords_pattern, r"(?i)\bRFC\s*(2119|8174)\b");
// An instruction to the reader: the verb starts the line, list item or
// sentence ("Read AGENTS.md first", "- See the `AGENTS.md` file", "Please
// follow AGENTS.md"), not prose about other tools ("for tools that read
// `AGENTS.md`").
static_regex!(fn agents_md_deferral_pattern, r"(?i)(?:^|[.!:;]\s+)\s*(?:[-*+]\s+|\d+[.)]\s+)?(?:\*\*)?(?:please\s+|always\s+|first,?\s+|you\s+(?:must|should)\s+|claude\s+(?:must|should)\s+)?(?P<ask>(?:read|see|follow|consult|check|refer\s+to|look\s+at)\b[^.\n]{0,60}?`?(?:\./)?AGENTS\.md`?)");
static_regex!(fn file_listing_item_pattern, r"^\s{0,6}[-*+]\s+(?:\*\*)?`?[A-Za-z0-9_.\-]+(?:/[A-Za-z0-9_.\-]+)*/?`?(?:\*\*)?\s*(?:-|–|—|:)\s+\S");
static_regex!(fn quoted_import_pattern, r#"(^|[\s(\[])@("([^"\n]+)"|'([^'\n]+)')"#);
static_regex!(fn spaced_import_pattern, r"(^|[\s(\[])@([A-Za-z0-9_.~/-]+)((?: [^\s@]+){1,4})");

/// CC-MEM-014: Claude Code's recommended maximum length of one instruction
/// file ("target under 200 lines per CLAUDE.md file").
pub const MAX_INSTRUCTION_FILE_LINES: usize = 200;

/// CC-MEM-015: agnix heuristic for the combined length of the instruction
/// files Claude Code loads at launch. Claude Code warns when files add up past
/// a combined limit but does not document the number, and the changelog says
/// the threshold scales with the context window.
pub const MAX_COMBINED_INSTRUCTION_LINES: usize = 400;

/// CC-MEM-016: MEMORY.md is loaded only up to "the first 200 lines ... or the
/// first 25KB, whichever comes first".
pub const MEMORY_INDEX_MAX_LINES: usize = 200;

/// CC-MEM-016: 25KB read limit. The docs do not say whether KB is 1,000 or
/// 1,024 bytes; the larger reading avoids false positives.
pub const MEMORY_INDEX_MAX_BYTES: usize = 25 * 1024;

/// CC-MEM-017: agnix heuristic. The best-practices page says to add emphasis
/// "to that line alone" and that "If you emphasize many lines, none of them
/// stands out"; it gives no count.
pub const EMPHASIS_LINE_LIMIT: usize = 5;

/// CC-MEM-021: agnix heuristic for a directory tree long enough to be a
/// layout Claude can derive from the code.
pub const DIRECTORY_TREE_MIN_LINES: usize = 20;

/// CC-MEM-021: agnix heuristic for a file-by-file listing.
pub const FILE_LISTING_MIN_ITEMS: usize = 15;

// ============================================================================
// Shared line handling
// ============================================================================

/// The fence that opens a fenced code block on `trimmed`, if any.
fn fence_marker(trimmed: &str) -> Option<(char, usize)> {
    let first = trimmed.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let len = trimmed.chars().take_while(|&c| c == first).count();
    (len >= 3).then_some((first, len))
}

/// Byte offset where the body of a file with YAML frontmatter starts, or 0.
/// Frontmatter opens with `---` on the first line and closes at the next line
/// that is exactly `---`.
pub fn frontmatter_body_start(content: &str) -> usize {
    let mut lines = content.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return 0;
    };
    if first.trim_end() != "---" {
        return 0;
    }
    let mut offset = first.len();
    for line in lines {
        offset += line.len();
        if line.trim_end() == "---" {
            return offset;
        }
    }
    0
}

/// How [`loaded_line_count`] treats a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCountMode {
    /// CLAUDE.md and CLAUDE.local.md: "Block-level HTML comments ... are
    /// stripped before the content is injected into Claude's context."
    MemoryFile,
    /// `.claude/rules/*.md`: "Claude Code removes the frontmatter before
    /// loading the rule into context."
    RuleFile,
    /// An imported file: counted as written.
    Plain,
}

/// Number of lines of `content` that reach Claude's context. Blank lines count:
/// the docs measure files in lines, not non-empty lines.
pub fn loaded_line_count(content: &str, mode: LineCountMode) -> usize {
    let body = if mode == LineCountMode::RuleFile {
        &content[frontmatter_body_start(content)..]
    } else {
        content
    };
    if mode != LineCountMode::MemoryFile {
        return body.lines().count();
    }

    let mut count = 0usize;
    let mut fence: Option<(char, usize)> = None;
    let mut in_comment = false;
    for line in body.lines() {
        if in_comment {
            if line.contains("-->") {
                in_comment = false;
            }
            continue;
        }
        let indent = line.len() - line.trim_start_matches(' ').len();
        let trimmed = line.trim();
        if let Some((ch, len)) = fence {
            count += 1;
            if fence_marker(trimmed).is_some_and(|(c, l)| c == ch && l >= len)
                && trimmed.chars().all(|c| c == ch)
            {
                fence = None;
            }
            continue;
        }
        if indent <= 3
            && let Some(marker) = fence_marker(trimmed)
        {
            fence = Some(marker);
            count += 1;
            continue;
        }
        if indent <= 3 && trimmed.starts_with("<!--") {
            match trimmed.find("-->") {
                // A one-line comment is stripped only when nothing follows it.
                Some(end) if trimmed[end + 3..].trim().is_empty() => continue,
                Some(_) => {}
                None => {
                    in_comment = true;
                    continue;
                }
            }
        }
        count += 1;
    }
    count
}

// ============================================================================
// CC-MEM-004: Invalid npm Script Reference
// ============================================================================

#[derive(Debug, Clone)]
pub struct NpmScriptReference {
    pub line: usize,
    pub column: usize,
    pub script_name: String,
}

/// Extract npm script references from content
pub fn extract_npm_scripts(content: &str) -> Vec<NpmScriptReference> {
    let mut results = Vec::new();
    let pattern = npm_run_pattern();

    for (line_num, line) in content.lines().enumerate() {
        for cap in pattern.captures_iter(line) {
            if let Some(script_match) = cap.get(1) {
                results.push(NpmScriptReference {
                    line: line_num + 1,
                    column: cap.get(0).map(|m| m.start() + 1).unwrap_or(1),
                    script_name: script_match.as_str().to_string(),
                });
            }
        }
    }

    results
}

// ============================================================================
// CC-MEM-010: README Duplication
// ============================================================================

/// Calculate text overlap between two texts as a percentage (0.0 - 1.0)
/// Uses word-set Jaccard similarity
pub fn calculate_text_overlap(text1: &str, text2: &str) -> f64 {
    // Normalize and extract words
    let text1_lower = text1.to_lowercase();
    let text2_lower = text2.to_lowercase();

    let words1: HashSet<&str> = text1_lower
        .split_whitespace()
        .filter(|w| w.len() > 3) // Skip short words
        .collect();

    let words2: HashSet<&str> = text2_lower
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .collect();

    if words1.is_empty() || words2.is_empty() {
        return 0.0;
    }

    // Jaccard similarity: intersection / union
    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Result when README duplication is detected
#[derive(Debug, Clone)]
pub struct ReadmeDuplication {
    pub overlap_percent: f64,
    pub threshold: f64,
}

/// Check if content duplicates README beyond threshold. The 40 percent
/// threshold is an agnix heuristic.
pub fn check_readme_duplication(claude_md: &str, readme: &str) -> Option<ReadmeDuplication> {
    let overlap = calculate_text_overlap(claude_md, readme);
    let threshold = 0.40; // 40% overlap threshold

    if overlap > threshold {
        Some(ReadmeDuplication {
            overlap_percent: overlap * 100.0,
            threshold: threshold * 100.0,
        })
    } else {
        None
    }
}

// ============================================================================
// CC-MEM-017: Emphasis Overuse
// ============================================================================

/// A line that uses all-caps emphasis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmphasisLine {
    pub line: usize,
    pub column: usize,
    pub token: String,
}

/// Lines outside code and HTML comments that contain an all-caps emphasis
/// token (IMPORTANT, CRITICAL, MUST, NEVER, ALWAYS, MANDATORY, DO NOT).
///
/// Returns nothing for documents that cite RFC 2119 or RFC 8174: those use
/// MUST and NEVER as defined requirement levels, not as emphasis.
pub fn find_emphasis_lines(content: &str) -> Vec<EmphasisLine> {
    let masked = mask_code_and_html_comments(content);
    if rfc_keywords_pattern().is_match(&masked) {
        return Vec::new();
    }
    let pattern = emphasis_token_pattern();
    masked
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let found = pattern.find(line)?;
            Some(EmphasisLine {
                line: index + 1,
                column: line[..found.start()].chars().count() + 1,
                token: found.as_str().to_string(),
            })
        })
        .collect()
}

// ============================================================================
// CC-MEM-018: CLAUDE.md Defers to AGENTS.md in Prose
// ============================================================================

/// A sentence that asks Claude to read AGENTS.md instead of importing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentsMdDeferral {
    pub line: usize,
    pub column: usize,
    pub text: String,
}

/// Find prose such as "Read AGENTS.md first" outside code blocks and
/// comments. Inline code is kept: "See the `AGENTS.md` file" is still prose.
pub fn find_agents_md_deferral(content: &str) -> Option<AgentsMdDeferral> {
    let masked = mask_code_and_html_comments(content);
    let pattern = agents_md_deferral_pattern();
    let original: Vec<&str> = content.lines().collect();
    masked.lines().enumerate().find_map(|(index, masked_line)| {
        let line = original.get(index).copied().unwrap_or_default();
        // A line with text that masking blanked entirely is code or comment.
        if masked_line.trim().is_empty() {
            return None;
        }
        let found = pattern.captures(line)?.name("ask")?;
        // `@AGENTS.md` is the import the docs recommend, not prose.
        if line[..found.end()].contains("@AGENTS.md") || line.contains("@./AGENTS.md") {
            return None;
        }
        Some(AgentsMdDeferral {
            line: index + 1,
            column: line[..found.start()].chars().count() + 1,
            text: found.as_str().trim().to_string(),
        })
    })
}

// ============================================================================
// CC-MEM-019: Quoted Import Path
// ============================================================================

/// An `@"path"` or `@'path'` token, which Claude Code does not import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotedImport {
    pub line: usize,
    pub column: usize,
    pub raw: String,
    pub path: String,
}

/// Quoted `@` imports outside code and comments whose content looks like a
/// file path (has a `/` or a file extension), so mentions of quoted people or
/// handles are not reported.
pub fn find_quoted_imports(content: &str) -> Vec<QuotedImport> {
    let masked = mask_code_and_html_comments(content);
    let mut found = Vec::new();
    for (index, line) in masked.lines().enumerate() {
        for cap in quoted_import_pattern().captures_iter(line) {
            let Some(whole) = cap.get(2) else { continue };
            let inner = cap
                .get(3)
                .or_else(|| cap.get(4))
                .map(|m| m.as_str())
                .unwrap_or_default();
            let looks_like_path = inner.contains('/')
                || inner
                    .rsplit_once('.')
                    .is_some_and(|(stem, ext)| !stem.is_empty() && (1..=5).contains(&ext.len()));
            if !looks_like_path {
                continue;
            }
            // Column of the `@`, one byte before the quoted text.
            let at = whole.start() - 1;
            found.push(QuotedImport {
                line: index + 1,
                column: line[..at].chars().count() + 1,
                raw: format!("@{}", whole.as_str()),
                path: inner.to_string(),
            });
        }
    }
    found
}

/// An `@name` followed by more words on the same line, which may be a path
/// with an unescaped space: "Without the backslashes, the path ends at the
/// first space".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpacedImport {
    pub line: usize,
    pub column: usize,
    /// The text Claude Code imports, up to the first space.
    pub first: String,
    /// The words that follow, in order.
    pub words: Vec<String>,
}

/// Candidates for [`SpacedImport`] outside code and comments. The caller
/// decides by checking which joined path exists.
pub fn find_spaced_imports(content: &str) -> Vec<SpacedImport> {
    let masked = mask_code_and_html_comments(content);
    let mut found = Vec::new();
    for (index, line) in masked.lines().enumerate() {
        for cap in spaced_import_pattern().captures_iter(line) {
            let (Some(first), Some(rest)) = (cap.get(2), cap.get(3)) else {
                continue;
            };
            let words = rest
                .as_str()
                .split(' ')
                .filter(|w| !w.is_empty())
                .map(|w| w.trim_end_matches(['.', ',', ';', ':', ')']).to_string())
                .collect();
            found.push(SpacedImport {
                line: index + 1,
                column: line[..first.start() - 1].chars().count() + 1,
                first: first.as_str().to_string(),
                words,
            });
        }
    }
    found
}

// ============================================================================
// CC-MEM-021: Directory Tree or File-by-File Listing
// ============================================================================

/// A block of consecutive lines that lists files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingBlock {
    /// 1-based first line of the block.
    pub line: usize,
    /// Number of lines (tree) or items (listing) in the block.
    pub size: usize,
}

fn is_tree_line(line: &str) -> bool {
    line.contains("├──") || line.contains("└──") || line.contains("|-- ") || line.contains("`-- ")
}

/// The longest run of directory-tree lines (`├──`, `└──`, `│`), in or out of
/// code fences, when it reaches [`DIRECTORY_TREE_MIN_LINES`].
pub fn find_directory_tree(content: &str) -> Option<ListingBlock> {
    longest_run(content, |line| {
        is_tree_line(line) || line.trim_start().starts_with('│')
    })
    .filter(|block| block.size >= DIRECTORY_TREE_MIN_LINES)
}

/// The longest run of list items of the form `- path/file.ext - description`
/// outside code, when it reaches [`FILE_LISTING_MIN_ITEMS`].
pub fn find_file_listing(content: &str) -> Option<ListingBlock> {
    let masked = crate::parsers::markdown::mask_code_spans(content);
    // Inline code is masked too, so match the original line at the same index.
    let original: Vec<&str> = content.lines().collect();
    let pattern = file_listing_item_pattern();
    let mut best: Option<ListingBlock> = None;
    let mut current: Option<ListingBlock> = None;
    for (index, masked_line) in masked.lines().enumerate() {
        let line = original.get(index).copied().unwrap_or_default();
        let in_code = masked_line.trim().is_empty() && !line.trim().is_empty();
        let item = !in_code
            && pattern.is_match(line)
            && line
                .split_whitespace()
                .nth(1)
                .is_some_and(|token| token.contains('/') || token.contains('.'));
        if item {
            let block = current.get_or_insert(ListingBlock {
                line: index + 1,
                size: 0,
            });
            block.size += 1;
            if best.as_ref().is_none_or(|b| block.size > b.size) {
                best = Some(block.clone());
            }
        } else if !line.trim().is_empty() {
            current = None;
        }
    }
    best.filter(|block| block.size >= FILE_LISTING_MIN_ITEMS)
}

fn longest_run(content: &str, mut matches: impl FnMut(&str) -> bool) -> Option<ListingBlock> {
    let mut best: Option<ListingBlock> = None;
    let mut current: Option<ListingBlock> = None;
    for (index, line) in content.lines().enumerate() {
        if matches(line) {
            let block = current.get_or_insert(ListingBlock {
                line: index + 1,
                size: 0,
            });
            block.size += 1;
            if best.as_ref().is_none_or(|b| block.size > b.size) {
                best = Some(block.clone());
            }
        } else {
            current = None;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_patterns_compile() {
        let _ = npm_run_pattern();
        let _ = emphasis_token_pattern();
        let _ = rfc_keywords_pattern();
        let _ = agents_md_deferral_pattern();
        let _ = file_listing_item_pattern();
        let _ = quoted_import_pattern();
        let _ = spaced_import_pattern();
    }

    // ===== Line counting =====

    #[test]
    fn test_loaded_line_count_counts_blank_lines() {
        // The docs say "lines"; blank lines take context too.
        let content = "a\n\nb\n\n\nc\n";
        assert_eq!(loaded_line_count(content, LineCountMode::MemoryFile), 6);
        assert_eq!(loaded_line_count(content, LineCountMode::Plain), 6);
    }

    #[test]
    fn test_loaded_line_count_skips_block_html_comments() {
        let content = "a\n<!-- one line -->\n<!--\nmaintainer\nnotes\n-->\nb\n";
        assert_eq!(loaded_line_count(content, LineCountMode::MemoryFile), 2);
        // Imports are counted as written.
        assert_eq!(loaded_line_count(content, LineCountMode::Plain), 7);
    }

    #[test]
    fn test_loaded_line_count_keeps_comments_in_code_and_inline() {
        let content = "```html\n<!-- kept -->\n```\ntext <!-- inline -->\n<!-- a --> trailing\n";
        assert_eq!(loaded_line_count(content, LineCountMode::MemoryFile), 5);
    }

    #[test]
    fn test_loaded_line_count_strips_rule_frontmatter() {
        let content = "---\npaths:\n  - src/**\n---\nrule one\nrule two\n";
        assert_eq!(loaded_line_count(content, LineCountMode::RuleFile), 2);
        assert_eq!(
            loaded_line_count("no frontmatter\n", LineCountMode::RuleFile),
            1
        );
        // An unclosed block is not frontmatter.
        assert_eq!(loaded_line_count("---\nx\n", LineCountMode::RuleFile), 2);
    }

    // ===== CC-MEM-004 =====

    #[test]
    fn test_extract_npm_scripts() {
        let content = "Run tests with npm run test\nBuild with npm run build";
        let results = extract_npm_scripts(content);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].script_name, "test");
        assert_eq!(results[1].script_name, "build");
    }

    #[test]
    fn test_extract_npm_scripts_with_colon() {
        let content = "Run npm run test:unit for unit tests";
        let results = extract_npm_scripts(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].script_name, "test:unit");
    }

    #[test]
    fn test_no_npm_scripts() {
        let content = "Use cargo test for testing.";
        let results = extract_npm_scripts(content);
        assert!(results.is_empty());
    }

    // ===== CC-MEM-010 =====

    #[test]
    fn test_calculate_text_overlap_identical() {
        let text = "This is some sample text with enough words to test overlap calculation.";
        let overlap = calculate_text_overlap(text, text);
        assert!((overlap - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_text_overlap_different() {
        let text1 = "This project uses Rust for performance.";
        let text2 = "Python is great for machine learning.";
        let overlap = calculate_text_overlap(text1, text2);
        assert!(overlap < 0.3);
    }

    #[test]
    fn test_check_readme_duplication_detected() {
        let claude_md =
            "This is a project about Rust validation. It validates agent configurations.";
        let readme = "This is a project about Rust validation. It validates agent configurations.";
        let result = check_readme_duplication(claude_md, readme);
        assert!(result.is_some());
    }

    #[test]
    fn test_check_readme_duplication_not_detected() {
        let claude_md = "Project-specific instructions for Claude. Focus on these guidelines.";
        let readme = "Welcome to the project. Installation: npm install. Usage: npm start.";
        let result = check_readme_duplication(claude_md, readme);
        assert!(result.is_none());
    }

    // ===== CC-MEM-017 =====

    #[test]
    fn test_find_emphasis_lines() {
        let content = "# Rules\nIMPORTANT: run tests.\nYou MUST lint.\nnever mind lowercase\n\
                       ```\nNEVER inside code\n```\n`ALWAYS` inline code\n<!-- CRITICAL note -->\nDO NOT push.\n";
        let lines = find_emphasis_lines(content);
        let numbers: Vec<_> = lines.iter().map(|l| l.line).collect();
        assert_eq!(numbers, vec![2, 3, 10]);
        assert_eq!(lines[1].token, "MUST");
        assert_eq!(lines[1].column, 5);
        assert_eq!(lines[2].token, "DO NOT");
    }

    #[test]
    fn test_find_emphasis_lines_skips_rfc_2119_documents() {
        let content = "The key words MUST and NEVER are to be interpreted as described in RFC 2119.\n\
                       Clients MUST validate.\n";
        assert!(find_emphasis_lines(content).is_empty());
    }

    #[test]
    fn test_find_emphasis_lines_ignores_words_containing_tokens() {
        assert!(find_emphasis_lines("MUSTARD and NEVERLAND and ALWAYSON\n").is_empty());
    }

    // ===== CC-MEM-018 =====

    #[test]
    fn test_find_agents_md_deferral() {
        let found = find_agents_md_deferral("# Project\nRead AGENTS.md before you start.\n")
            .expect("deferral");
        assert_eq!(found.line, 2);
        assert_eq!(found.column, 1);
        assert!(
            find_agents_md_deferral("See the `AGENTS.md` file for conventions.\n").is_some(),
            "a backticked name is still prose that defers to the file"
        );
    }

    #[test]
    fn test_find_agents_md_deferral_skips_imports_and_code() {
        assert!(find_agents_md_deferral("@AGENTS.md\n").is_none());
        assert!(find_agents_md_deferral("Follow @AGENTS.md for conventions.\n").is_none());
        assert!(find_agents_md_deferral("```\nread AGENTS.md\n```\n").is_none());
        assert!(find_agents_md_deferral("AGENTS.md is generated from this file.\n").is_none());
        // Describing other tools is not an instruction to Claude.
        assert!(
            find_agents_md_deferral(
                "- `AGENTS.md` is a copy of `CLAUDE.md` for tools that read `AGENTS.md`.\n"
            )
            .is_none()
        );
        assert!(find_agents_md_deferral("Setup done. Please read AGENTS.md.\n").is_some());
        assert!(find_agents_md_deferral("1. **Follow AGENTS.md** for style.\n").is_some());
    }

    // ===== CC-MEM-019 =====

    #[test]
    fn test_find_quoted_imports() {
        let content =
            "- API @\"Design Docs/api.md\"\n- see @'notes.md'\n- ask @'alice'\n`@\"code/x.md\"`\n";
        let found = find_quoted_imports(content);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].path, "Design Docs/api.md");
        assert_eq!(found[0].line, 1);
        assert_eq!(found[0].column, 7);
        assert_eq!(found[1].raw, "@'notes.md'");
    }

    #[test]
    fn test_find_spaced_imports() {
        let found = find_spaced_imports("- API conventions @Design Docs/api.md\n`@x y`\n");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].first, "Design");
        assert_eq!(found[0].words, vec!["Docs/api.md".to_string()]);
        assert_eq!(found[0].column, 19);
        // An escaped space is one path, which the import parser handles.
        let escaped = find_spaced_imports("@Design\\ Docs/api.md\n");
        assert!(escaped.iter().all(|s| s.first != "Design"));
    }

    // ===== CC-MEM-021 =====

    #[test]
    fn test_find_directory_tree_threshold() {
        let mut tree = vec!["```".to_string(), "repo/".to_string()];
        for i in 0..DIRECTORY_TREE_MIN_LINES {
            tree.push(format!("├── file{i}.rs"));
        }
        tree.push("```".to_string());
        let block = find_directory_tree(&tree.join("\n")).expect("tree");
        assert_eq!(block.line, 3);
        assert_eq!(block.size, DIRECTORY_TREE_MIN_LINES);

        let short = "├── a\n└── b\n";
        assert!(find_directory_tree(short).is_none());
    }

    #[test]
    fn test_find_file_listing_threshold() {
        let items: Vec<String> = (0..FILE_LISTING_MIN_ITEMS)
            .map(|i| format!("- `src/mod{i}.rs` - module {i}"))
            .collect();
        let block = find_file_listing(&items.join("\n")).expect("listing");
        assert_eq!(block.size, FILE_LISTING_MIN_ITEMS);
        assert!(find_file_listing(&items[..3].join("\n")).is_none());
        // Ordinary bullet points are not file entries.
        let prose: Vec<String> = (0..20)
            .map(|i| format!("- Rule {i}: be specific"))
            .collect();
        assert!(find_file_listing(&prose.join("\n")).is_none());
    }
}
