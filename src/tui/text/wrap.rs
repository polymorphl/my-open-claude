//! Text wrapping for display.

/// Split text into lines of max width (columns). Uses textwrap for correct UTF-8 handling.
fn wrap_text(s: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![s.to_string()];
    }
    textwrap::wrap(s, width)
        .into_iter()
        .map(|cow| cow.into_owned())
        .collect()
}

/// Split a message into display lines respecting message newlines, then wrap to `width`.
pub(crate) fn wrap_message(msg: &str, width: usize) -> Vec<String> {
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
