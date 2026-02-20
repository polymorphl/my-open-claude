//! Centralized keyboard shortcuts.
//!
//! Complete reference:
//!
//! | Action        | Keys                                    |
//! |---------------|-----------------------------------------|
//! | Send          | Enter                                    |
//! | Newline       | Shift+Enter                              |
//! | Scroll        | ↑ ↓ PageUp PageDown                     |
//! | History       | Alt+H, Esc+h (Option as meta), Mac chars |
//! | New conv      | Ctrl+N                                      |
//! | Model selector| Alt+M, Esc+m, µ (Option+M Mac)          |
//! | Copy message  | ⌘C (macOS) / Ctrl+Shift+C (Linux, Windows) |
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
    /// New conversation (Ctrl+N)
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
const MAC_OPTION_M: char = '\u{00B5}'; // µ

fn is_mac_option_h(c: char) -> bool {
    MAC_OPTION_H.contains(&c)
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
                KeyCode::Char('m') => Some(Shortcut::ModelSelector),
                _ => None,
            };
        }

        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Shortcut::Quit)
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Shortcut::NewConversation)
            }
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::ALT) => {
                Some(Shortcut::History)
            }
            KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::ALT) => {
                Some(Shortcut::ModelSelector)
            }
            KeyCode::Char(c) if is_mac_option_h(c) => Some(Shortcut::History),
            KeyCode::Char(MAC_OPTION_M) => Some(Shortcut::ModelSelector),
            _ => None,
        }
    }

    /// True if key is Escape (start of Option+key sequence on some terminals).
    pub fn is_escape(key: &KeyEvent) -> bool {
        key.kind == KeyEventKind::Press && key.code == KeyCode::Esc
    }
}

#[cfg(test)]
mod tests {
    use super::Shortcut;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    #[test]
    fn is_escape() {
        assert!(Shortcut::is_escape(&key(
            KeyCode::Esc,
            KeyModifiers::empty()
        )));
        assert!(!Shortcut::is_escape(&key(
            KeyCode::Char('c'),
            KeyModifiers::empty()
        )));
    }

    #[test]
    fn match_quit_ctrl_c() {
        assert_eq!(
            Shortcut::match_key(&key(KeyCode::Char('c'), KeyModifiers::CONTROL), false),
            Some(Shortcut::Quit)
        );
    }

    #[test]
    fn match_history_alt_h() {
        assert_eq!(
            Shortcut::match_key(&key(KeyCode::Char('h'), KeyModifiers::ALT), false),
            Some(Shortcut::History)
        );
    }

    #[test]
    fn match_model_selector_alt_m() {
        assert_eq!(
            Shortcut::match_key(&key(KeyCode::Char('m'), KeyModifiers::ALT), false),
            Some(Shortcut::ModelSelector)
        );
    }

    #[test]
    fn match_new_conversation_ctrl_n() {
        assert_eq!(
            Shortcut::match_key(&key(KeyCode::Char('n'), KeyModifiers::CONTROL), false),
            Some(Shortcut::NewConversation)
        );
    }

    #[test]
    fn match_escape_pending_h() {
        assert_eq!(
            Shortcut::match_key(&key(KeyCode::Char('h'), KeyModifiers::empty()), true),
            Some(Shortcut::History)
        );
    }

    #[test]
    fn match_escape_pending_m() {
        assert_eq!(
            Shortcut::match_key(&key(KeyCode::Char('m'), KeyModifiers::empty()), true),
            Some(Shortcut::ModelSelector)
        );
    }

    #[test]
    fn match_no_shortcut() {
        assert_eq!(
            Shortcut::match_key(&key(KeyCode::Char('x'), KeyModifiers::empty()), false),
            None
        );
    }

    #[test]
    fn match_key_release_ignored() {
        let key_release = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Release,
            state: KeyEventState::empty(),
        };
        assert_eq!(Shortcut::match_key(&key_release, false), None);
    }
}

/// Labels for the bottom bar (2 lines for readability on narrow terminals).
pub mod labels {
    use ratatui::style::Color;
    use ratatui::text::{Line, Span, Text};

    const DIM: Color = Color::DarkGray;

    #[cfg(target_os = "macos")]
    const COPY_KEY: &str = "  ⌘C ";
    #[cfg(not(target_os = "macos"))]
    const COPY_KEY: &str = "  Ctrl+Shift+C ";

    pub fn bottom_bar(is_streaming: bool) -> Text<'static> {
        if is_streaming {
            Text::from(Line::from(vec![
                Span::styled("Esc ", Color::Yellow),
                Span::raw("cancel"),
                Span::styled("  ↑↓ ", DIM),
                Span::raw("scroll"),
            ]))
        } else {
            Text::from(vec![
                Line::from(vec![
                    Span::styled("Enter ", DIM),
                    Span::raw("send"),
                    Span::styled("  Shift/Alt+Enter ", DIM),
                    Span::raw("newline"),
                    Span::styled("  Ctrl+U ", DIM),
                    Span::raw("clear"),
                    Span::styled("  ↑↓ ", DIM),
                    Span::raw("scroll"),
                ]),
                Line::from(vec![
                    Span::styled("Alt+H ", DIM),
                    Span::raw("history"),
                    Span::styled("  Ctrl+N ", DIM),
                    Span::raw("new"),
                    Span::styled("  Alt+M ", DIM),
                    Span::raw("model"),
                    Span::styled(COPY_KEY, DIM),
                    Span::raw("copy"),
                    Span::styled("  Ctrl+C ", DIM),
                    Span::raw("quit"),
                ]),
            ])
        }
    }
}
