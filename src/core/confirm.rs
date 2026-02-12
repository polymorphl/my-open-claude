//! Confirmation of destructive actions (e.g. Bash commands like rm, rmdir).
//! Used by CLI (prompt mode). The TUI uses an in-app popup instead.

/// Callback type for confirming destructive Bash commands.
/// Receives the command, returns true to run, false to cancel.
/// Sync required so futures holding &ConfirmDestructive across await points are Send.
pub type ConfirmDestructive = Box<dyn Fn(&str) -> bool + Send + Sync>;

/// Default implementation: prompt on stderr, read y/N from stdin.
/// For CLI (prompt mode) where the terminal is already in cooked mode.
pub fn default_confirm() -> ConfirmDestructive {
    Box::new(|cmd: &str| {
        eprintln!("⚠ Destructive command: {}", cmd);
        eprint!("Confirm? [y/N] ");
        let _ = std::io::Write::flush(&mut std::io::stderr());
        let mut s = String::new();
        let _ = std::io::stdin().read_line(&mut s);
        let t = s.trim();
        t.eq_ignore_ascii_case("y") || t.eq_ignore_ascii_case("yes")
    })
}
