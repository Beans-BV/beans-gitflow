use std::io::{self, Write};
use crossterm::{
    cursor, event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{self, Stylize},
    terminal,
};
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

    // Print prompt
    execute!(
        out,
        style::PrintStyledContent("? ".green().bold()),
        style::Print(prompt),
        style::Print("\n"),
    ).map_err(|e| format!("Menu error: {e}"))?;

    terminal::enable_raw_mode().map_err(|e| format!("Menu error: {e}"))?;

    // Hide cursor during selection
    execute!(out, cursor::Hide).map_err(|e| {
        let _ = terminal::disable_raw_mode();
        format!("Menu error: {e}")
    })?;

    // Initial render
    render_menu(&mut out, items, selected).map_err(|e| {
        let _ = execute!(out, cursor::Show);
        let _ = terminal::disable_raw_mode();
        format!("Menu error: {e}")
    })?;

    let result = loop {
        let ev = event::read().map_err(|e| {
            let _ = execute!(out, cursor::Show);
            let _ = terminal::disable_raw_mode();
            format!("Menu error: {e}")
        })?;

        match ev {
            Event::Key(KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, .. }) |
            Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => {
                let _ = execute!(out, cursor::Show);
                let _ = terminal::disable_raw_mode();
                return Err("Aborted".to_string());
            }
            Event::Key(KeyEvent { code: KeyCode::Enter, .. }) => {
                break selected;
            }
            Event::Key(KeyEvent { code: KeyCode::Up, .. }) => {
                if selected > 0 {
                    selected -= 1;
                }
            }
            Event::Key(KeyEvent { code: KeyCode::Down, .. }) => {
                if selected < items.len() - 1 {
                    selected += 1;
                }
            }
            Event::Key(KeyEvent { code: KeyCode::Char(c), modifiers: KeyModifiers::NONE, .. }) => {
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
        let reposition = (|| -> io::Result<()> {
            if items.len() > 1 {
                execute!(out, cursor::MoveUp((items.len() - 1) as u16))?;
            }
            execute!(out, cursor::MoveToColumn(0))?;
            Ok(())
        })();
        if let Err(e) = reposition {
            let _ = execute!(out, cursor::Show);
            let _ = terminal::disable_raw_mode();
            return Err(format!("Menu error: {e}"));
        }
        render_menu(&mut out, items, selected).map_err(|e| {
            let _ = execute!(out, cursor::Show);
            let _ = terminal::disable_raw_mode();
            format!("Menu error: {e}")
        })?;
    };

    // Cleanup: show cursor, disable raw mode, move past menu
    let _ = execute!(out, cursor::Show, style::Print("\r\n"));
    let _ = terminal::disable_raw_mode();

    Ok(result)
}

fn validate_branch_name(input: &str) -> Result<(), String> {
    if input.is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    if input.contains("..") || input.contains('~') || input.contains('^') || input.contains(':') || input.contains('\\') {
        return Err("Invalid branch name. Avoid special characters (.. ~ ^ : \\)".to_string());
    }
    Ok(())
}

pub fn prompt_name(prompt: &str) -> Result<String, String> {
    loop {
        let mut out = io::stderr();
        let mut input = String::new();

        // Print prompt
        execute!(
            out,
            style::PrintStyledContent("? ".green().bold()),
            style::Print(format!("{prompt}: ")),
        ).map_err(|e| format!("Input error: {e}"))?;

        terminal::enable_raw_mode().map_err(|e| format!("Input error: {e}"))?;

        let result = loop {
            let ev = event::read().map_err(|e| {
                let _ = terminal::disable_raw_mode();
                format!("Input error: {e}")
            })?;

            match ev {
                Event::Key(KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, .. }) |
                Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => {
                    let _ = terminal::disable_raw_mode();
                    return Err("Aborted".to_string());
                }
                Event::Key(KeyEvent { code: KeyCode::Enter, .. }) => {
                    break input.clone();
                }
                Event::Key(KeyEvent { code: KeyCode::Backspace, .. }) => {
                    if input.pop().is_some() {
                        let _ = execute!(
                            out,
                            cursor::MoveLeft(1),
                            style::Print(" "),
                            cursor::MoveLeft(1),
                        );
                    }
                }
                Event::Key(KeyEvent { code: KeyCode::Char(c), .. }) => {
                    let ch = if c == ' ' { '-' } else { c };
                    // Collapse consecutive hyphens: skip if last char is already '-' and new char is '-'
                    if ch == '-' && input.ends_with('-') {
                        continue;
                    }
                    input.push(ch);
                    let _ = execute!(out, style::Print(ch));
                }
                _ => {}
            }
        };

        let _ = execute!(out, cursor::MoveToNextLine(1));
        let _ = terminal::disable_raw_mode();

        // Trim leading/trailing hyphens
        let trimmed = result.trim_matches('-').to_string();

        match validate_branch_name(&trimmed) {
            Ok(()) => return Ok(trimmed),
            Err(e) => {
                let _ = execute!(
                    out,
                    style::PrintStyledContent(format!("  {e}").red()),
                    cursor::MoveToNextLine(1),
                );
                // Loop to re-prompt
            }
        }
    }
}

pub fn show_menu(branch_type: &BranchType, current_branch: &str) -> Result<Action, String> {
    match branch_type {
        BranchType::Main => {
            let labels = &["start hotfix fix"];
            show_select("What would you like to do?", labels)?;
            Ok(Action::StartHotfixFix)
        }
        BranchType::Develop => {
            let labels: Vec<&str> = DevelopOption::ALL.iter().map(|o| o.label()).collect();
            let idx = show_select("What would you like to do?", &labels)?;
            let option = DevelopOption::ALL[idx];
            match option {
                DevelopOption::StartRelease => Ok(Action::StartRelease),
                other => {
                    let name = prompt_name(&format!("Name for {} branch", other.branch_prefix()))?;
                    Ok(Action::StartWorkBranch { prefix: other.branch_prefix().to_string(), name, from: "develop".to_string() })
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
                WorkBranchOption::Finish => Ok(Action::FinishWorkBranch),
                other => {
                    let name = prompt_name(&format!("Name for {} branch", other.branch_prefix()))?;
                    let current_label = format!("{current_branch} (current)");
                    let base_options: &[&str] = &[&current_label, "develop"];
                    let base_idx = show_select("Base branch", base_options)?;
                    let from = if base_idx == 0 { current_branch.to_string() } else { "develop".to_string() };
                    Ok(Action::StartWorkBranch { prefix: other.branch_prefix().to_string(), name, from })
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
                ReleaseOption::StartReleaseFix => Ok(Action::StartReleaseFix),
                ReleaseOption::BumpVersion => Ok(Action::BumpVersion),
                ReleaseOption::SyncWithDevelop => Ok(Action::SyncWithDevelop),
                ReleaseOption::FinishRelease => Ok(Action::FinishRelease),
            }
        }
        BranchType::HotfixFix { .. } => {
            show_select("What would you like to do?", &["finish hotfix fix"])?;
            Ok(Action::FinishHotfixFix)
        }
        BranchType::Hotfix { .. } => Ok(Action::FinishHotfix),
        BranchType::Other => Err("Not on a recognized gitflow branch. Switch to main or develop first.".to_string()),
    }
}

#[derive(Debug)]
pub enum Action {
    StartWorkBranch { prefix: String, name: String, from: String },
    StartRelease,
    StartReleaseFix,
    StartHotfixFix,
    FinishWorkBranch,
    FinishReleaseFix,
    FinishRelease,
    FinishHotfix,
    FinishHotfixFix,
    BumpVersion,
    SyncWithDevelop,
}
