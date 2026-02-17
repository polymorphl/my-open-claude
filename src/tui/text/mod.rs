//! Text utilities: markdown parsing and line wrapping for the chat display.

mod markdown;
mod segments;
mod wrap;

pub(crate) use markdown::parse_markdown_inline;
pub(crate) use segments::{MessageSegment, parse_message_segments};
pub(crate) use wrap::wrap_message;

/// Normalize Unicode symbols to ASCII equivalents in code blocks.
/// LLMs sometimes output ≠, ≥, ≤ etc. instead of !=, >=, <= — this restores valid syntax.
pub(crate) fn normalize_code_operators(s: &str) -> String {
    s.replace('\u{2260}', "!=") // ≠ -> !=
        .replace('\u{2265}', ">=") // ≥ -> >=
        .replace('\u{2264}', "<=") // ≤ -> <=
}

#[cfg(test)]
mod tests;
