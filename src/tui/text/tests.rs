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
fn parse_markdown_inline_bullet_list() {
    let spans = parse_markdown_inline("- item one");
    assert!(spans.len() >= 2);
    assert_eq!(spans[0].content.as_ref(), "• ");
}

#[test]
fn parse_markdown_inline_numbered_list() {
    let spans = parse_markdown_inline("1. first");
    assert!(spans.len() >= 2);
}

#[test]
fn parse_markdown_inline_link() {
    let spans = parse_markdown_inline("See [docs](https://example.com) for more.");
    assert!(spans.len() >= 2);
}

#[test]
fn parse_markdown_inline_table_row() {
    let spans = parse_markdown_inline("| name | value |");
    assert!(!spans.is_empty());
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
