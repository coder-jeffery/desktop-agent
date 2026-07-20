use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("path escapes workspace: {0}")]
    EscapesWorkspace(String),
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct WorkspaceSandbox {
    root: PathBuf,
}

impl WorkspaceSandbox {
    pub fn new(root: PathBuf) -> Result<Self, SandboxError> {
        if !root.is_dir() {
            return Err(SandboxError::InvalidPath(format!(
                "not a directory: {}",
                root.display()
            )));
        }
        let canonical = root.canonicalize()?;
        Ok(Self { root: canonical })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn resolve(&self, relative: &str) -> Result<PathBuf, SandboxError> {
        let trimmed = relative.trim().trim_start_matches("./");
        if trimmed.is_empty() {
            return Ok(self.root.clone());
        }
        if Path::new(trimmed).is_absolute() {
            return Err(SandboxError::EscapesWorkspace(
                "absolute paths are not allowed".into(),
            ));
        }
        let joined = self.root.join(trimmed);
        self.ensure_within(&joined)
    }

    pub fn ensure_within(&self, path: &Path) -> Result<PathBuf, SandboxError> {
        let canonical = if path.exists() {
            path.canonicalize()?
        } else {
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(&self.root);
            let parent_canonical = if parent.exists() {
                parent.canonicalize()?
            } else {
                self.root.clone()
            };
            let file_name = path
                .file_name()
                .ok_or_else(|| SandboxError::InvalidPath(path.display().to_string()))?;
            parent_canonical.join(file_name)
        };

        if !canonical.starts_with(&self.root) {
            return Err(SandboxError::EscapesWorkspace(
                canonical.display().to_string(),
            ));
        }
        Ok(canonical)
    }

    pub fn relative_display(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.display().to_string())
    }
}

pub fn is_shell_command_blocked(command: &str) -> Option<&'static str> {
    let lower = command.to_lowercase();
    let blocked = [
        ("rm -rf /", "recursive delete of root"),
        ("rm -rf /*", "recursive delete of root"),
        ("mkfs", "filesystem format"),
        ("dd if=", "raw disk write"),
        (":(){ :|:& };:", "fork bomb"),
        ("chmod -r /", "recursive chmod on root"),
        ("> /etc/", "write to /etc"),
        ("| sh", "pipe to shell"),
        ("| bash", "pipe to shell"),
    ];

    for (pattern, reason) in blocked {
        if lower.contains(pattern) {
            return Some(reason);
        }
    }
    None
}
