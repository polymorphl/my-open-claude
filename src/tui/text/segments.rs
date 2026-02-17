//! Message segmentation: split content into text and fenced code blocks.

/// Segment of a message: either plain text or a fenced code block.
#[derive(Debug, Clone)]
pub(crate) enum MessageSegment<'a> {
    Text(&'a str),
    CodeBlock { lang: &'a str, code: &'a str },
}

/// Parse message content into text and code block segments.
/// Matches ```lang ... ``` or ``` ... ``` patterns.
pub(crate) fn parse_message_segments(content: &str) -> Vec<MessageSegment<'_>> {
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
                // Closing ``` can be: "\n```" (on its own line) or "```" (no newline before)
                let end = rest.find("\n```").or_else(|| rest.find("```"));
                match end {
                    Some(pos) => {
                        let (code, after) =
                            if rest.get(pos..).is_some_and(|s| s.starts_with("\n```")) {
                                (&rest[..pos], &rest[pos + 4..])
                            } else {
                                (&rest[..pos], &rest[pos + 3..])
                            };
                        segments.push(MessageSegment::CodeBlock { lang, code });
                        rest = after;
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
