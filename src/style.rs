//! Terminal styling policy. One rule, one place: style only when a human is
//! looking (stdout is a TTY) and has not opted out (NO_COLOR, TERM=dumb —
//! see clig.dev and no-color.org). The decision is pure and tested; only the
//! environment reads live in the untestable shell.

/// Whether styled output is appropriate, given the environment.
/// `no_color` is the NO_COLOR variable (any non-empty value disables styling);
/// `term` is TERM (`dumb` disables styling).
pub fn styling_enabled(is_tty: bool, no_color: Option<&str>, term: Option<&str>) -> bool {
    is_tty && no_color.unwrap_or("").is_empty() && term != Some("dumb")
}

/// The production answer: resolves the environment and applies the rule.
pub fn styled() -> bool {
    use std::io::IsTerminal;
    styling_enabled(
        std::io::stdout().is_terminal(),
        std::env::var("NO_COLOR").ok().as_deref(),
        std::env::var("TERM").ok().as_deref(),
    )
}

/// ANSI-bold `text` when `enabled`, else `text` unchanged.
pub fn bold(text: &str, enabled: bool) -> String {
    if enabled {
        format!("\x1b[1m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styling_needs_a_tty_and_no_opt_out() {
        assert!(styling_enabled(true, None, Some("xterm-256color")));
        assert!(styling_enabled(true, None, None));
    }

    #[test]
    fn no_tty_means_no_styling() {
        assert!(!styling_enabled(false, None, Some("xterm-256color")));
    }

    #[test]
    fn any_nonempty_no_color_disables_styling() {
        // no-color.org: any non-empty value counts, even "0" or "false".
        assert!(!styling_enabled(true, Some("1"), None));
        assert!(!styling_enabled(true, Some("false"), None));
        assert!(styling_enabled(true, Some(""), None), "empty NO_COLOR is unset");
    }

    #[test]
    fn dumb_terminal_disables_styling() {
        assert!(!styling_enabled(true, None, Some("dumb")));
    }

    #[test]
    fn bold_wraps_only_when_enabled() {
        assert_eq!(bold("title", true), "\x1b[1mtitle\x1b[0m");
        assert_eq!(bold("title", false), "title");
    }
}
