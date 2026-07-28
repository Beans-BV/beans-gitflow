use std::path::Path;
use std::process::Command;

use crate::git::Result;

/// Opens a directory in an editor. Abstracted as a trait so the worktree flow can
/// be unit-tested without launching a real editor.
pub trait Editor {
    fn open(&self, path: &Path) -> Result<()>;
}

/// Opens a path by running `<command> <path>`. Works for any editor whose CLI opens
/// a folder this way (e.g. `code`, `cursor`).
pub struct CommandEditor {
    command: String,
}

impl CommandEditor {
    pub fn new(command: String) -> Self {
        Self { command }
    }
}

impl Editor for CommandEditor {
    fn open(&self, path: &Path) -> Result<()> {
        let status = Command::new(&self.command)
            .arg(path)
            .status()
            .map_err(|e| format!("failed to run editor '{}': {e}", self.command))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("editor '{}' exited with {}", self.command, status))
        }
    }
}
