//! Byte-to-position utilities for LSP.
//!
//! Converts byte offsets from agnix-core's Fix struct to LSP Position/Range
//! types. LSP defaults to UTF-16 code unit positions, while agnix-core uses
//! byte offsets for precise text manipulation.

use tower_lsp::lsp_types::{Position, Range};

/// Convert a byte offset to an LSP Position (line, UTF-16 code units).
///
/// The position is 0-indexed for both line and character, matching LSP conventions.
/// Handles UTF-8 correctly by iterating over character boundaries.
///
/// # Arguments
///
/// * `content` - The full file content
/// * `byte_offset` - Byte offset into the content
///
/// # Returns
///
/// An LSP Position with line and UTF-16 character fields.
pub fn byte_to_position(content: &str, byte_offset: usize) -> Position {
    let mut line = 0u32;
    let mut character = 0u32;
    let mut current_byte = 0usize;

    for c in content.chars() {
        if current_byte >= byte_offset {
            break;
        }

        if c == '\n' {
            line += 1;
            character = 0;
        } else {
            character += c.len_utf16() as u32;
        }

        current_byte += c.len_utf8();
    }

    Position { line, character }
}

/// Convert an LSP Position (line, UTF-16 code units) to a byte offset.
///
/// The returned offset is clamped to valid UTF-8 boundaries.
pub fn position_to_byte(content: &str, position: Position) -> usize {
    let mut current_line = 0u32;
    let mut line_start = 0usize;

    for (idx, ch) in content.char_indices() {
        if current_line == position.line {
            line_start = idx;
            break;
        }
        if ch == '\n' {
            current_line += 1;
            line_start = idx + 1;
            if current_line == position.line {
                break;
            }
        }
    }

    if current_line < position.line {
        return content.len();
    }

    let line_tail = &content[line_start..];
    let line_end = line_tail
        .find('\n')
        .map(|idx| line_start + idx)
        .unwrap_or(content.len());
    let line_content = &content[line_start..line_end];

    utf16_position_to_byte_in_line(line_content, position.character)
        .map(|byte_idx| line_start + byte_idx)
        .unwrap_or(line_end)
}

/// Convert a UTF-16 code unit offset within one line to a byte offset.
pub(crate) fn utf16_position_to_byte_in_line(line: &str, character: u32) -> Option<usize> {
    let mut utf16_count = 0u32;
    for (byte_idx, ch) in line.char_indices() {
        if utf16_count == character {
            return Some(byte_idx);
        }

        let next = utf16_count + ch.len_utf16() as u32;
        if next > character {
            return Some(byte_idx);
        }

        utf16_count = next;
    }

    if utf16_count == character {
        Some(line.len())
    } else {
        None
    }
}

/// Convert a byte range to an LSP Range.
///
/// Creates a Range from start and end byte offsets. Both positions are
/// calculated using [`byte_to_position`].
///
/// # Arguments
///
/// * `content` - The full file content
/// * `start_byte` - Start byte offset (inclusive)
/// * `end_byte` - End byte offset (exclusive)
///
/// # Returns
///
/// An LSP Range with start and end positions.
pub fn byte_range_to_lsp_range(content: &str, start_byte: usize, end_byte: usize) -> Range {
    Range {
        start: byte_to_position(content, start_byte),
        end: byte_to_position(content, end_byte),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_to_position_start() {
        let content = "hello";
        let pos = byte_to_position(content, 0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_byte_to_position_same_line() {
        let content = "hello world";
        let pos = byte_to_position(content, 6); // 'w'
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 6);
    }

    #[test]
    fn test_byte_to_position_second_line() {
        let content = "hello\nworld";
        let pos = byte_to_position(content, 6); // 'w' on line 2
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_byte_to_position_middle_of_second_line() {
        let content = "hello\nworld";
        let pos = byte_to_position(content, 8); // 'r' in "world"
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 2);
    }

    #[test]
    fn test_byte_to_position_multiple_lines() {
        let content = "line1\nline2\nline3";
        let pos = byte_to_position(content, 12); // 'l' in "line3"
        assert_eq!(pos.line, 2);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_byte_to_position_end_of_content() {
        let content = "hello";
        let pos = byte_to_position(content, 5); // past the end
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 5);
    }

    #[test]
    fn test_byte_to_position_empty_content() {
        let content = "";
        let pos = byte_to_position(content, 0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_byte_to_position_utf8_multibyte() {
        // UTF-8 multibyte character test
        let content = "hello\u{00e9}world"; // e with acute accent (2 bytes)
        // "hello" is 5 bytes, e-acute is 2 bytes, so 'w' is at byte 7
        let pos = byte_to_position(content, 7);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 6); // 6 characters: h-e-l-l-o-e
    }

    #[test]
    fn test_byte_to_position_non_bmp_uses_utf16_units() {
        let content = "\u{1f600}name";
        let pos = byte_to_position(content, 4);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 2);
    }

    #[test]
    fn test_byte_to_position_crlf() {
        // Windows line endings (we count \r as a character before \n)
        let content = "hello\r\nworld";
        let pos = byte_to_position(content, 7); // 'w' after CRLF
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_byte_range_to_lsp_range_same_line() {
        let content = "hello world";
        let range = byte_range_to_lsp_range(content, 0, 5); // "hello"
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 5);
    }

    #[test]
    fn test_byte_range_to_lsp_range_cross_line() {
        let content = "hello\nworld";
        let range = byte_range_to_lsp_range(content, 3, 8); // "lo\nwo"
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 3);
        assert_eq!(range.end.line, 1);
        assert_eq!(range.end.character, 2);
    }

    #[test]
    fn test_byte_range_to_lsp_range_insertion_point() {
        // start == end (insertion point)
        let content = "hello";
        let range = byte_range_to_lsp_range(content, 5, 5);
        assert_eq!(range.start, range.end);
        assert_eq!(range.start.character, 5);
    }

    #[test]
    fn test_byte_range_yaml_frontmatter() {
        // Typical YAML frontmatter scenario
        let content = "---\nname: test-skill\nversion: 1.0.0\n---\n";
        // "name: " starts at byte 4, value "test-skill" is bytes 10-20
        let range = byte_range_to_lsp_range(content, 10, 20);
        assert_eq!(range.start.line, 1);
        assert_eq!(range.start.character, 6); // after "name: "
        assert_eq!(range.end.line, 1);
        assert_eq!(range.end.character, 16); // end of "test-skill"
    }

    #[test]
    fn test_position_to_byte_same_line() {
        let content = "hello world";
        let byte = position_to_byte(
            content,
            Position {
                line: 0,
                character: 6,
            },
        );
        assert_eq!(byte, 6);
    }

    #[test]
    fn test_position_to_byte_multiline() {
        let content = "hello\nworld";
        let byte = position_to_byte(
            content,
            Position {
                line: 1,
                character: 2,
            },
        );
        assert_eq!(byte, 8);
    }

    #[test]
    fn test_position_to_byte_utf8() {
        let content = "aéz";
        let byte = position_to_byte(
            content,
            Position {
                line: 0,
                character: 2,
            },
        );
        // 'a' = 1 byte, 'é' = 2 bytes
        assert_eq!(byte, 3);
    }

    #[test]
    fn test_position_to_byte_non_bmp_uses_utf16_units() {
        let content = "name: \u{1f600}x";
        let byte = position_to_byte(
            content,
            Position {
                line: 0,
                character: 8,
            },
        );
        assert_eq!(byte, 10);
    }
}
