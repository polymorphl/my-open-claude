//! Centralized keyboard shortcuts.
//!
//! Complete reference:
//!
//! | Action        | Keys                                    |
//! |---------------|-----------------------------------------|
//! | Send          | Enter                                    |
//! | Scroll        | ↑ ↓ PageUp PageDown                     |
//! | History       | Alt+H, Esc+h (Option as meta), Mac chars |
//! | New conv      | Alt+N, Esc+n, Mac chars                  |
//! | Model selector| Alt+M, Esc+m, µ (Option+M Mac)          |
//! | Quit          | Ctrl+C                                   |
//!
//! On macOS, Option+key can send:
//! - Esc+key if terminal has "Use option as meta key" enabled
//! - A special character (˙, ˜, µ) if Option is in normal mode

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Detected shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shortcut {
    /// Open conversation history (Alt+H, Esc+h)
    History,
    /// New conversation (Alt+N, Esc+n)
    NewConversation,
    /// Model selector (Alt+M, Esc+m)
    ModelSelector,
    /// Quit (Ctrl+C)
    Quit,
    /// No shortcut
    None,
}

/// Characters produced by Option+key on Mac (Option not configured as Meta).
/// Varies by terminal/keyboard. Option+H = Ì (U+00CC), Option+N = ~ (U+007E), Option+M = µ (U+00B5).
const MAC_OPTION_H: &[char] = &['\u{00CC}', '\u{02D9}', '\u{0127}', '\u{0302}']; // Ì, ˙, ħ, ̂
const MAC_OPTION_N: &[char] = &['\u{007E}', '\u{02DC}', '\u{0303}', '\u{0144}', '\u{0148}', '\u{00F1}']; // ~, ˜, ̃, ń, ň, ñ
const MAC_OPTION_M: char = '\u{00B5}'; // µ

fn is_mac_option_h(c: char) -> bool {
    MAC_OPTION_H.contains(&c)
}

fn is_mac_option_n(c: char) -> bool {
    MAC_OPTION_N.contains(&c)
}

impl Shortcut {
    /// Returns the shortcut if the key matches. Handles Esc+key sequence when terminal
    /// sends Option as Meta (e.g. macOS "Use option as meta key").
    pub fn match_key(key: &KeyEvent, escape_pending: bool) -> Option<Shortcut> {
        if key.kind != KeyEventKind::Press {
            return None;
        }

        if escape_pending {
            return match key.code {
                KeyCode::Char('h') => Some(Shortcut::History),
                KeyCode::Char('n') => Some(Shortcut::NewConversation),
                KeyCode::Char('m') => Some(Shortcut::ModelSelector),
                _ => None,
            };
        }

        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Shortcut::Quit)
            }
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::ALT) => {
                Some(Shortcut::History)
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::ALT) => {
                Some(Shortcut::NewConversation)
            }
            KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::ALT) => {
                Some(Shortcut::ModelSelector)
            }
            KeyCode::Char(c) if is_mac_option_h(c) => Some(Shortcut::History),
            KeyCode::Char(c) if is_mac_option_n(c) => Some(Shortcut::NewConversation),
            KeyCode::Char(MAC_OPTION_M) => Some(Shortcut::ModelSelector),
            _ => None,
        }
    }

    /// True if key is Escape (start of Option+key sequence on some terminals).
    pub fn is_escape(key: &KeyEvent) -> bool {
        key.kind == KeyEventKind::Press && key.code == KeyCode::Esc
    }
}

/// Labels for the bottom bar.
pub mod labels {
    use ratatui::style::Color;
    use ratatui::text::{Line, Span};

    const DIM: Color = Color::DarkGray;

    pub fn bottom_bar(is_streaming: bool) -> Line<'static> {
        if is_streaming {
            Line::from(vec![
                Span::styled("Esc ", Color::Yellow),
                Span::raw("cancel"),
                Span::styled("  ↑↓ ", DIM),
                Span::raw("scroll"),
            ])
        } else {
            Line::from(vec![
                Span::styled("Enter ", DIM),
                Span::raw("send"),
                Span::styled("  ↑↓ ", DIM),
                Span::raw("scroll"),
                Span::styled("  Alt+H ", DIM),
                Span::raw("history"),
                Span::styled("  Alt+N ", DIM),
                Span::raw("new"),
                Span::styled("  Alt+M ", DIM),
                Span::raw("model"),
                Span::styled("  Ctrl+C ", DIM),
                Span::raw("quit"),
            ])
        }
    }
}
