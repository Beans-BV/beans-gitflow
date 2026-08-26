//! Port for interactive input. Business logic that needs a user decision — the
//! branch-type menu, the worktree wizard, the release-type and breaking-change
//! prompts — depends on this trait instead of the terminal-UI module, so it
//! stays runnable against mocks without a TTY. The real implementation is
//! `menu::MenuPrompter`, wired in `main.rs` like the other adapters.
//!
//! Three methods, one actor: the human at the terminal. `select` chooses from a
//! list; the two readers differ in how they shape what is typed (see
//! decisions.md, "Input shaping over validation"), which is why `prompt_name`
//! owns its re-prompt loop rather than returning invalid input to the caller.

pub trait Prompter {
    /// Present `items` and return the index of the chosen one.
    fn select(&self, prompt: &str, items: &[&str]) -> Result<usize, String>;

    /// Read a branch name, re-prompting until it passes `validate_branch_name`.
    /// Input is shaped as it is typed (space → `-`, collapsed, trimmed).
    fn prompt_name(&self, prompt: &str) -> Result<String, String>;

    /// Read a free-form line — paths and commands — with no shaping and no
    /// validation beyond trimming.
    fn prompt_line(&self, prompt: &str) -> Result<String, String>;
}
