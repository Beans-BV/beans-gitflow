use std::io::{self, Write};
use crossterm::{
    cursor, event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{self, Stylize},
    terminal,
};
use crate::flows::start::ReleaseType;
use crate::git::branch::BranchType;

#[derive(Debug, Clone, Copy)]
pub enum DevelopOption {
    StartFeature, StartFix, StartChore, StartDocs, StartRefactor, StartRelease,
}

impl DevelopOption {
    pub fn label(&self) -> &'static str {
        match self {
            Self::StartFeature => "start feature",
            Self::StartFix => "start fix",
            Self::StartChore => "start chore",
            Self::StartDocs => "start docs",
            Self::StartRefactor => "start refactor",
            Self::StartRelease => "start release",
        }
    }

    pub fn branch_prefix(&self) -> &'static str {
        match self {
            Self::StartFeature => "feature",
            Self::StartFix => "fix",
            Self::StartChore => "chore",
            Self::StartDocs => "docs",
            Self::StartRefactor => "refactor",
            Self::StartRelease => unreachable!(),
        }
    }

    const ALL: [Self; 6] = [Self::StartFeature, Self::StartFix, Self::StartChore, Self::StartDocs, Self::StartRefactor, Self::StartRelease];
}

#[derive(Debug, Clone, Copy)]
pub enum WorkBranchOption {
    Finish, StartFeature, StartFix, StartChore, StartDocs, StartRefactor,
}

impl WorkBranchOption {
    pub fn label(&self, branch_type: &str) -> String {
        match self {
            Self::Finish => format!("finish {branch_type}"),
            Self::StartFeature => "start feature".to_string(),
            Self::StartFix => "start fix".to_string(),
            Self::StartChore => "start chore".to_string(),
            Self::StartDocs => "start docs".to_string(),
            Self::StartRefactor => "start refactor".to_string(),
        }
    }

    pub fn branch_prefix(&self) -> &'static str {
        match self {
            Self::StartFeature => "feature",
            Self::StartFix => "fix",
            Self::StartChore => "chore",
            Self::StartDocs => "docs",
            Self::StartRefactor => "refactor",
            Self::Finish => unreachable!(),
        }
    }

    const ALL: [Self; 6] = [Self::Finish, Self::StartFeature, Self::StartFix, Self::StartChore, Self::StartDocs, Self::StartRefactor];
}

#[derive(Debug, Clone, Copy)]
pub enum ReleaseOption {
    FinishRelease, StartReleaseFix, BumpVersion, SyncWithDevelop,
}

impl ReleaseOption {
    pub fn label(&self) -> &'static str {
        match self {
            Self::FinishRelease => "finish release",
            Self::StartReleaseFix => "start release fix",
            Self::BumpVersion => "bump version",
            Self::SyncWithDevelop => "sync with develop",
        }
    }

    const ALL: [Self; 4] = [Self::FinishRelease, Self::StartReleaseFix, Self::BumpVersion, Self::SyncWithDevelop];
}

/// Enables raw mode on construction; restores the terminal (cursor visible, raw
/// mode off) on drop. Every exit path — success, error, Ctrl-C/Esc abort — runs
/// the same cleanup structurally, so the documented "no raw-mode leak" invariant
/// is enforced by the type, not by hand-written cleanup at each return.
struct TerminalGuard;

impl TerminalGuard {
    fn enter(context: &str) -> Result<Self, String> {
        terminal::enable_raw_mode().map_err(|e| format!("{context}: {e}"))?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stderr(), cursor::Show);
        let _ = terminal::disable_raw_mode();
    }
}

fn render_menu(out: &mut io::Stderr, items: &[&str], selected: usize) -> io::Result<()> {
    for (i, item) in items.iter().enumerate() {
        let number = i + 1;
        queue!(out, cursor::MoveToColumn(0), terminal::Clear(terminal::ClearType::CurrentLine))?;
        if i == selected {
            queue!(
                out,
                style::PrintStyledContent(format!("> {number}) {item}").cyan().bold()),
            )?;
        } else {
            queue!(
                out,
                style::PrintStyledContent(format!("  {number}) {item}").dim()),
            )?;
        }
        if i < items.len() - 1 {
            queue!(out, style::Print("\r\n"))?;
        }
    }
    out.flush()?;
    Ok(())
}

pub fn show_select(prompt: &str, items: &[&str]) -> Result<usize, String> {
    if items.is_empty() {
        return Err("Menu error: no items to select from".to_string());
    }

    let mut out = io::stderr();
    let mut selected: usize = 0;

    // Print prompt (clear line first to avoid ghost text from previous prompts)
    execute!(
        out,
        cursor::MoveToColumn(0),
        terminal::Clear(terminal::ClearType::CurrentLine),
        style::PrintStyledContent("? ".green().bold()),
        style::Print(prompt),
        style::Print("\n"),
    ).map_err(|e| format!("Menu error: {e}"))?;

    let _guard = TerminalGuard::enter("Menu error")?;

    // Hide cursor during selection; the guard re-shows it on every exit path.
    execute!(out, cursor::Hide).map_err(|e| format!("Menu error: {e}"))?;

    // Initial render
    render_menu(&mut out, items, selected).map_err(|e| format!("Menu error: {e}"))?;

    let result = loop {
        let ev = event::read().map_err(|e| format!("Menu error: {e}"))?;

        // On Windows, crossterm emits Press + Release events; only handle Press
        let Event::Key(KeyEvent { kind: KeyEventKind::Press, code, modifiers, .. }) = ev else {
            continue;
        };

        match (code, modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Esc, _) => {
                return Err("Aborted".to_string());
            }
            (KeyCode::Enter, _) => {
                break selected;
            }
            (KeyCode::Up, _) => {
                if selected > 0 {
                    selected -= 1;
                }
            }
            (KeyCode::Down, _) => {
                if selected < items.len() - 1 {
                    selected += 1;
                }
            }
            (KeyCode::Char(c), KeyModifiers::NONE) => {
                if let Some(digit) = c.to_digit(10) {
                    let idx = digit as usize;
                    if idx >= 1 && idx <= items.len() && idx <= 9 {
                        selected = idx - 1;
                        // Re-render to show selection highlighted before returning
                        if items.len() > 1 {
                            let _ = execute!(out, cursor::MoveUp((items.len() - 1) as u16));
                        }
                        let _ = execute!(out, cursor::MoveToColumn(0));
                        let _ = render_menu(&mut out, items, selected);
                        break selected;
                    }
                }
            }
            _ => {}
        }

        // Redraw: move cursor up to start of menu, then re-render
        if items.len() > 1 {
            execute!(out, cursor::MoveUp((items.len() - 1) as u16))
                .map_err(|e| format!("Menu error: {e}"))?;
        }
        execute!(out, cursor::MoveToColumn(0)).map_err(|e| format!("Menu error: {e}"))?;
        render_menu(&mut out, items, selected).map_err(|e| format!("Menu error: {e}"))?;
    };

    // Move past the menu; the guard restores cursor + raw mode on drop.
    let _ = execute!(out, style::Print("\r\n"));

    Ok(result)
}

pub fn validate_branch_name(input: &str) -> Result<(), String> {
    if input.is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    if input.contains("..") || input.contains('~') || input.contains('^') || input.contains(':') || input.contains('\\') {
        return Err("Invalid branch name. Avoid special characters (.. ~ ^ : \\)".to_string());
    }
    Ok(())
}

/// Print `prompt` and read a line of input in raw mode. Shared scaffolding for
/// `prompt_name`/`prompt_line`: prompt printing, raw-mode lifecycle, the
/// Windows Press-filter, Ctrl-C/Esc abort, Enter, and backspace handling all
/// live here. `transform` decides which char (if any) each keystroke appends,
/// given the buffer typed so far.
fn read_raw_line(prompt: &str, transform: impl Fn(&str, char) -> Option<char>) -> Result<String, String> {
    let mut out = io::stderr();
    let mut input = String::new();

    execute!(
        out,
        style::PrintStyledContent("? ".green().bold()),
        style::Print(format!("{prompt}: ")),
    ).map_err(|e| format!("Input error: {e}"))?;

    let _guard = TerminalGuard::enter("Input error")?;

    let result = loop {
        let ev = event::read().map_err(|e| format!("Input error: {e}"))?;

        // On Windows, crossterm emits Press + Release events; only handle Press
        let Event::Key(KeyEvent { kind: KeyEventKind::Press, code, modifiers, .. }) = ev else {
            continue;
        };

        match (code, modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Esc, _) => {
                return Err("Aborted".to_string());
            }
            (KeyCode::Enter, _) => break input,
            (KeyCode::Backspace, _) => {
                if input.pop().is_some() {
                    let _ = execute!(out, cursor::MoveLeft(1), style::Print(" "), cursor::MoveLeft(1));
                }
            }
            (KeyCode::Char(c), _) => {
                if let Some(ch) = transform(&input, c) {
                    input.push(ch);
                    let _ = execute!(out, style::Print(ch));
                }
            }
            _ => {}
        }
    };

    // Move to the next line; the guard disables raw mode on drop.
    let _ = execute!(out, cursor::MoveToNextLine(1));
    Ok(result)
}

pub fn prompt_name(prompt: &str) -> Result<String, String> {
    loop {
        // Spaces become hyphens as you type; consecutive hyphens collapse.
        let result = read_raw_line(prompt, |input, c| {
            let ch = if c == ' ' { '-' } else { c };
            if ch == '-' && input.ends_with('-') { None } else { Some(ch) }
        })?;

        // Trim leading/trailing hyphens
        let trimmed = result.trim_matches('-').to_string();

        match validate_branch_name(&trimmed) {
            Ok(()) => return Ok(trimmed),
            Err(e) => {
                let _ = execute!(
                    io::stderr(),
                    style::PrintStyledContent(format!("  {e}").red()),
                    cursor::MoveToNextLine(1),
                );
                // Loop to re-prompt
            }
        }
    }
}

/// Prompt for a free-form line of text (spaces, slashes, `~` all allowed, no
/// validation). Unlike `prompt_name`, this does not mangle input into a branch
/// name — use it for paths and shell commands.
pub fn prompt_line(prompt: &str) -> Result<String, String> {
    Ok(read_raw_line(prompt, |_, c| Some(c))?.trim().to_string())
}

pub fn show_menu(branch_type: &BranchType, current_branch: &str) -> Result<Action, String> {
    match branch_type {
        BranchType::Main => {
            let labels = &["start hotfix fix"];
            show_select("What would you like to do?", labels)?;
            let name = prompt_name("Name for hotfix-fix branch")?;
            Ok(Action::StartHotfixFix { name, no_checkout: false, no_worktree: false })
        }
        BranchType::Develop => {
            let labels: Vec<&str> = DevelopOption::ALL.iter().map(|o| o.label()).collect();
            let idx = show_select("What would you like to do?", &labels)?;
            let option = DevelopOption::ALL[idx];
            match option {
                DevelopOption::StartRelease => Ok(Action::StartRelease(None)),
                other => {
                    let name = prompt_name(&format!("Name for {} branch", other.branch_prefix()))?;
                    Ok(Action::StartWorkBranch { prefix: other.branch_prefix().to_string(), name, from: "develop".to_string(), no_checkout: false, no_worktree: false })
                }
            }
        }
        BranchType::Feature { .. } | BranchType::Fix { .. } | BranchType::Chore { .. }
        | BranchType::Docs { .. } | BranchType::Refactor { .. } => {
            let branch_type_label = match branch_type {
                BranchType::Feature { .. } => "feature",
                BranchType::Fix { .. } => "fix",
                BranchType::Chore { .. } => "chore",
                BranchType::Docs { .. } => "docs",
                BranchType::Refactor { .. } => "refactor",
                _ => unreachable!(),
            };
            let labels: Vec<String> = WorkBranchOption::ALL.iter().map(|o| o.label(branch_type_label)).collect();
            let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            let idx = show_select("What would you like to do?", &label_refs)?;
            let option = WorkBranchOption::ALL[idx];
            match option {
                WorkBranchOption::Finish => Ok(Action::FinishWorkBranch { breaking: None, base: None }),
                other => {
                    let name = prompt_name(&format!("Name for {} branch", other.branch_prefix()))?;
                    let current_label = format!("{current_branch} (current)");
                    let base_options: &[&str] = &[&current_label, "develop"];
                    let base_idx = show_select("Base branch", base_options)?;
                    let from = if base_idx == 0 { current_branch.to_string() } else { "develop".to_string() };
                    Ok(Action::StartWorkBranch { prefix: other.branch_prefix().to_string(), name, from, no_checkout: false, no_worktree: false })
                }
            }
        }
        BranchType::ReleaseFix { .. } => {
            show_select("What would you like to do?", &["finish release fix"])?;
            Ok(Action::FinishReleaseFix)
        }
        BranchType::Release { .. } => {
            let labels: Vec<&str> = ReleaseOption::ALL.iter().map(|o| o.label()).collect();
            let idx = show_select("What would you like to do?", &labels)?;
            match ReleaseOption::ALL[idx] {
                ReleaseOption::StartReleaseFix => {
                    let name = prompt_name("Name for release-fix branch")?;
                    Ok(Action::StartReleaseFix { name, no_checkout: false, no_worktree: false })
                }
                ReleaseOption::BumpVersion => Ok(Action::BumpVersion),
                ReleaseOption::SyncWithDevelop => Ok(Action::SyncWithDevelop),
                ReleaseOption::FinishRelease => Ok(Action::FinishRelease),
            }
        }
        BranchType::HotfixFix { .. } => {
            show_select("What would you like to do?", &["finish hotfix fix"])?;
            Ok(Action::FinishHotfixFix)
        }
        BranchType::Hotfix { .. } => {
            show_select("What would you like to do?", &["finish hotfix"])?;
            Ok(Action::FinishHotfix)
        }
        BranchType::Other => Err("Not on a recognized gitflow branch. Switch to main or develop first.".to_string()),
    }
}

#[derive(Debug, PartialEq)]
pub enum Action {
    StartWorkBranch { prefix: String, name: String, from: String, no_checkout: bool, no_worktree: bool },
    StartRelease(Option<ReleaseType>),
    StartReleaseFix { name: String, no_checkout: bool, no_worktree: bool },
    StartHotfixFix { name: String, no_checkout: bool, no_worktree: bool },
    FinishWorkBranch { breaking: Option<bool>, base: Option<String> },
    FinishReleaseFix,
    FinishRelease,
    FinishHotfix,
    FinishHotfixFix,
    AbortFinish,
    BumpVersion,
    SyncWithDevelop,
}

impl Action {
    pub fn is_start(&self) -> bool {
        matches!(
            self,
            Action::StartWorkBranch { .. }
                | Action::StartRelease(_)
                | Action::StartReleaseFix { .. }
                | Action::StartHotfixFix { .. }
        )
    }

    pub fn no_checkout(&self) -> bool {
        match self {
            Action::StartWorkBranch { no_checkout, .. } => *no_checkout,
            Action::StartReleaseFix { no_checkout, .. } => *no_checkout,
            Action::StartHotfixFix { no_checkout, .. } => *no_checkout,
            _ => false,
        }
    }

    /// Whether this action is a named-work-branch start eligible for the worktree
    /// flow. Deliberately excludes `StartRelease`, unlike `is_start`.
    pub fn worktree_eligible(&self) -> bool {
        matches!(
            self,
            Action::StartWorkBranch { .. }
                | Action::StartReleaseFix { .. }
                | Action::StartHotfixFix { .. }
        )
    }

    pub fn no_worktree(&self) -> bool {
        match self {
            Action::StartWorkBranch { no_worktree, .. } => *no_worktree,
            Action::StartReleaseFix { no_worktree, .. } => *no_worktree,
            Action::StartHotfixFix { no_worktree, .. } => *no_worktree,
            _ => false,
        }
    }
}
