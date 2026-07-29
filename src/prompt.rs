//! Port for interactive selection. Flows depend on this trait instead of the
//! terminal-UI module, so business logic that needs a user decision stays
//! runnable against mocks without a TTY. The real implementation is
//! `menu::MenuPrompter`, wired in `main.rs` like the other adapters.

pub trait Prompter {
    /// Present `items` and return the index of the chosen one.
    fn select(&self, prompt: &str, items: &[&str]) -> Result<usize, String>;
}
