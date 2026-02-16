//! Text utilities: markdown parsing and line wrapping for the chat display.

use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use super::constants::ACCENT;

/// Segment of a message: either plain text or a fenced code block.
#[derive(Debug, Clone)]
pub(super) enum MessageSegment<'a> {
    Text(&'a str),
    CodeBlock { lang: &'a str, code: &'a str },
}

/// Parse message content into text and code block segments.
/// Matches ```lang ... ``` or ``` ... ``` patterns.
pub(super) fn parse_message_segments(content: &str) -> Vec<MessageSegment<'_>> {
    let mut segments = Vec::new();
    let mut rest = content;
    loop {
        match rest.find("```") {
            None => {
                if !rest.is_empty() {
                    segments.push(MessageSegment::Text(rest));
                }
                break;
            }
            Some(idx) => {
                if idx > 0 {
                    let text = &rest[..idx];
                    segments.push(MessageSegment::Text(text));
                }
                rest = &rest[idx + 3..];
                let lang_end = rest.find('\n').unwrap_or(rest.len());
                let lang = rest[..lang_end].trim();
                rest = if lang_end < rest.len() {
                    &rest[lang_end + 1..]
                } else {
                    ""
                };
                match rest.find("\n```") {
                    Some(end) => {
                        let code = &rest[..end];
                        segments.push(MessageSegment::CodeBlock { lang, code });
                        rest = &rest[end + 4..];
                    }
                    None => {
                        segments.push(MessageSegment::CodeBlock { lang, code: rest });
                        break;
                    }
                }
            }
        }
    }
    segments
}

/// Parse inline Markdown: **bold**, `code`, and headings (# / ## / ###) at line start.
pub(super) fn parse_markdown_inline(s: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let trimmed = s.trim_start();
    // Heading: starts with one or more #
    if trimmed.starts_with('#') {
        let content = trimmed.trim_start_matches('#').trim_start();
        if !content.is_empty() {
            spans.push(Span::styled(
                content.to_string(),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ));
        }
        return spans;
    }
    // Otherwise parse ** and ` in the rest
    let mut rest = s;
    while !rest.is_empty() {
        let next_bold = rest.find("**");
        let next_code = rest.find('`');
        let (use_bold, pos) = match (next_bold, next_code) {
            (Some(b), None) => (true, b),
            (None, Some(c)) => (false, c),
            (Some(b), Some(c)) => (b <= c, b.min(c)),
            (None, None) => {
                spans.push(Span::raw(rest.to_string()));
                break;
            }
        };
        if pos > 0 {
            spans.push(Span::raw(rest[..pos].to_string()));
        }
        rest = &rest[pos..];
        if use_bold && rest.starts_with("**") {
            rest = &rest[2..];
            if let Some(end) = rest.find("**") {
                spans.push(Span::styled(
                    rest[..end].to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                rest = &rest[end + 2..];
            } else {
                spans.push(Span::raw("**".to_string()));
            }
        } else if rest.starts_with('`') {
            rest = &rest[1..];
            if let Some(end) = rest.find('`') {
                spans.push(Span::styled(
                    rest[..end].to_string(),
                    Style::default().fg(ACCENT),
                ));
                rest = &rest[end + 1..];
            } else {
                spans.push(Span::raw("`".to_string()));
            }
        }
    }
    spans
}

/// Split text into lines of max byte width `width`, only cutting at UTF-8 character boundaries.
fn wrap_text(s: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![s.to_string()];
    }
    let mut out = Vec::new();
    let mut rest = s;
    while !rest.is_empty() {
        if rest.len() <= width {
            out.push(rest.to_string());
            break;
        }
        // Last byte index that is a character boundary and <= width.
        let boundary = rest
            .char_indices()
            .rfind(|(i, _)| *i <= width)
            .map(|(i, _)| i);
        let boundary = match boundary {
            Some(b) if b > 0 => b,
            _ => {
                // First character exceeds width, cut after the first character.
                rest.char_indices()
                    .nth(1)
                    .map(|(i, _)| i)
                    .unwrap_or(rest.len())
            }
        };
        let (chunk, next) = if let Some(space) = rest[..boundary].rfind(' ') {
            let chunk = rest[..space].trim_end();
            let next = rest[space..].trim_start();
            (chunk, next)
        } else {
            (&rest[..boundary], rest[boundary..].trim_start())
        };
        out.push(chunk.to_string());
        rest = next;
    }
    out
}

/// Split a message into display lines respecting message newlines, then wrap to `width`.
pub(super) fn wrap_message(msg: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for line in msg.split('\n') {
        if line.is_empty() {
            out.push(String::new());
        } else {
            for chunk in wrap_text(line, width) {
                out.push(chunk);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{MessageSegment, parse_markdown_inline, parse_message_segments, wrap_message};

    #[test]
    fn parse_message_segments_empty() {
        let segs = parse_message_segments("");
        assert!(segs.is_empty());
    }

    #[test]
    fn parse_message_segments_text_only() {
        let segs = parse_message_segments("Hello world");
        assert_eq!(segs.len(), 1);
        assert!(matches!(&segs[0], MessageSegment::Text("Hello world")));
    }

    #[test]
    fn parse_message_segments_single_code_block() {
        let segs = parse_message_segments("```rust\nfn main() {}\n```");
        assert_eq!(segs.len(), 1);
        match &segs[0] {
            MessageSegment::CodeBlock { lang, code } => {
                assert_eq!(*lang, "rust");
                assert_eq!(*code, "fn main() {}");
            }
            _ => panic!("expected CodeBlock"),
        }
    }

    #[test]
    fn parse_message_segments_code_block_without_lang() {
        let segs = parse_message_segments("```\nfn main() {}\n```");
        assert_eq!(segs.len(), 1);
        match &segs[0] {
            MessageSegment::CodeBlock { lang, code } => {
                assert!(lang.is_empty());
                assert_eq!(*code, "fn main() {}");
            }
            _ => panic!("expected CodeBlock"),
        }
    }

    #[test]
    fn parse_message_segments_unclosed_code_block() {
        let segs = parse_message_segments("```rust\nfn main() {");
        assert_eq!(segs.len(), 1);
        match &segs[0] {
            MessageSegment::CodeBlock { lang, code } => {
                assert_eq!(*lang, "rust");
                assert_eq!(*code, "fn main() {");
            }
            _ => panic!("expected CodeBlock"),
        }
    }

    #[test]
    fn parse_message_segments_text_and_code() {
        let segs = parse_message_segments("Here is the fix:\n\n```rust\nlet x = 1;\n```\n\nDone.");
        assert_eq!(segs.len(), 3);
        assert!(matches!(&segs[0], MessageSegment::Text(t) if t.contains("Here is the fix")));
        assert!(matches!(&segs[1], MessageSegment::CodeBlock { lang, .. } if *lang == "rust"));
        assert!(matches!(&segs[2], MessageSegment::Text(t) if t.contains("Done.")));
    }

    #[test]
    fn parse_message_segments_multiple_code_blocks() {
        let segs = parse_message_segments("```a\n1\n```\n\n```b\n2\n```");
        assert_eq!(segs.len(), 3);
        assert!(
            matches!(&segs[0], MessageSegment::CodeBlock { lang, code } if *lang == "a" && *code == "1")
        );
        assert!(matches!(&segs[1], MessageSegment::Text(t) if *t == "\n\n"));
        assert!(
            matches!(&segs[2], MessageSegment::CodeBlock { lang, code } if *lang == "b" && *code == "2")
        );
    }

    #[test]
    fn parse_markdown_inline_plain() {
        let spans = parse_markdown_inline("hello");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content.as_ref(), "hello");
    }

    #[test]
    fn parse_markdown_inline_bold() {
        use ratatui::style::Modifier;
        let spans = parse_markdown_inline("**bold** text");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content.as_ref(), "bold");
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[1].content.as_ref(), " text");
    }

    #[test]
    fn parse_markdown_inline_inline_code() {
        let spans = parse_markdown_inline("Use `println!` macro");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content.as_ref(), "Use ");
        assert_eq!(spans[1].content.as_ref(), "println!");
        assert_eq!(spans[2].content.as_ref(), " macro");
    }

    #[test]
    fn parse_markdown_inline_heading() {
        let spans = parse_markdown_inline("## Section");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content.as_ref(), "Section");
    }

    #[test]
    fn wrap_message_preserves_newlines() {
        let lines = wrap_message("line1\nline2", 100);
        assert_eq!(lines, ["line1", "line2"]);
    }

    #[test]
    fn wrap_message_wraps_long_line() {
        let lines = wrap_message("hello world test", 8);
        assert_eq!(lines, ["hello", "world", "test"]);
    }

    #[test]
    fn wrap_message_empty_lines() {
        let lines = wrap_message("a\n\nb", 100);
        assert_eq!(lines, ["a", "", "b"]);
    }
}
