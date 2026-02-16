//! Syntax highlighting for code blocks using syntect.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

use super::constants::ACCENT_SECONDARY;

/// Convert syntect Color to ratatui Color. Alpha 0 => None (colourless).
fn translate_colour(c: syntect::highlighting::Color) -> Option<Color> {
    let syntect::highlighting::Color { r, g, b, a } = c;
    if a > 0 {
        Some(Color::Rgb(r, g, b))
    } else {
        None
    }
}

/// Convert syntect FontStyle to ratatui Modifier. Supports BOLD, ITALIC, UNDERLINE and combinations.
fn translate_font_style(f: FontStyle) -> Modifier {
    if f.is_empty() {
        return Modifier::empty();
    }
    let mut m = Modifier::empty();
    if f.contains(FontStyle::BOLD) {
        m.insert(Modifier::BOLD);
    }
    if f.contains(FontStyle::ITALIC) {
        m.insert(Modifier::ITALIC);
    }
    if f.contains(FontStyle::UNDERLINE) {
        m.insert(Modifier::UNDERLINED);
    }
    m
}

/// Convert syntect Style to ratatui Style.
fn translate_style(s: syntect::highlighting::Style) -> Style {
    let fg = translate_colour(s.foreground).unwrap_or(ACCENT_SECONDARY);
    let bg = translate_colour(s.background);
    let modifier = translate_font_style(s.font_style);
    let mut style = Style::default().fg(fg).add_modifier(modifier);
    if let Some(b) = bg {
        style = style.bg(b);
    }
    style
}

static SYNTAX_SET: std::sync::OnceLock<SyntaxSet> = std::sync::OnceLock::new();
static THEME_SET: std::sync::OnceLock<ThemeSet> = std::sync::OnceLock::new();

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

/// Map language identifier from markdown (e.g. "rust", "python") to syntect extension.
fn lang_to_extension(lang: &str) -> &'static str {
    match lang.trim().to_lowercase().as_str() {
        "rs" | "rust" => "rs",
        "py" | "python" => "py",
        "js" | "javascript" => "js",
        "ts" | "typescript" => "ts",
        "go" | "golang" => "go",
        "rb" | "ruby" => "rb",
        "sh" | "bash" | "zsh" => "sh",
        "sql" => "sql",
        "json" => "json",
        "yaml" | "yml" => "yml",
        "toml" => "toml",
        "md" | "markdown" => "md",
        "html" => "html",
        "css" => "css",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        _ => "plain",
    }
}

/// Highlight a single line of code. Returns styled spans, or a plain span on error/unknown lang.
pub(super) fn highlight_code_line(lang: &str, line: &str) -> Vec<Span<'static>> {
    if lang.trim().is_empty() || lang_to_extension(lang) == "plain" {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(ACCENT_SECONDARY),
        )];
    }

    let ps = syntax_set();
    let ts = theme_set();

    let syntax = match ps.find_syntax_by_extension(lang_to_extension(lang)) {
        Some(s) => s,
        None => {
            return vec![Span::styled(
                line.to_string(),
                Style::default().fg(ACCENT_SECONDARY),
            )];
        }
    };

    let theme = ts
        .themes
        .get("base16-ocean.dark")
        .or_else(|| ts.themes.values().next())
        .expect("at least one theme");

    let mut h = HighlightLines::new(syntax, theme);
    let line_with_ending = if line.ends_with('\n') {
        line.to_string()
    } else {
        format!("{}\n", line)
    };

    let segments = match h.highlight_line(line_with_ending.as_str(), ps) {
        Ok(segments) => segments,
        Err(_) => {
            return vec![Span::styled(
                line.to_string(),
                Style::default().fg(ACCENT_SECONDARY),
            )];
        }
    };

    let mut result = Vec::new();
    for (style, content) in segments {
        let s = content.to_string();
        if s.is_empty() {
            continue;
        }
        result.push(Span::styled(s, translate_style(style)));
    }
    result
}

/// Slice spans to cover only the character range [range_start, range_end).
/// Used when wrapping code lines: each wrap chunk gets the spans for its character slice.
pub(super) fn slice_spans_by_range(
    spans: &[Span<'static>],
    range_start: usize,
    range_end: usize,
) -> Vec<Span<'static>> {
    let mut result = Vec::new();
    let mut pos = 0;
    for span in spans {
        let s = span.content.as_ref();
        let len = s.chars().count();
        let span_end = pos + len;
        if span_end <= range_start || pos >= range_end {
            pos = span_end;
            continue;
        }
        let take_start = range_start.saturating_sub(pos);
        let take_end = (range_end - pos).min(len);
        if take_start < take_end {
            let sliced: String = s
                .chars()
                .skip(take_start)
                .take(take_end - take_start)
                .collect();
            result.push(Span::styled(sliced, span.style));
        }
        pos = span_end;
    }
    result
}
