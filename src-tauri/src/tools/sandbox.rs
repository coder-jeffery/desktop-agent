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

    /// Resolve a path that may not exist yet, preserving nested segments under the workspace.
    pub fn ensure_within(&self, path: &Path) -> Result<PathBuf, SandboxError> {
        if path.exists() {
            let canonical = path.canonicalize()?;
            if !canonical.starts_with(&self.root) {
                return Err(SandboxError::EscapesWorkspace(
                    canonical.display().to_string(),
                ));
            }
            return Ok(canonical);
        }

        let mut missing = Vec::new();
        let mut cursor = path;
        loop {
            if cursor.exists() {
                break;
            }
            let name = cursor.file_name().ok_or_else(|| {
                SandboxError::InvalidPath(path.display().to_string())
            })?;
            missing.push(name.to_os_string());
            match cursor.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => cursor = parent,
                _ => break,
            }
        }

        let base = if cursor.exists() {
            cursor.canonicalize()?
        } else {
            self.root.clone()
        };

        if !base.starts_with(&self.root) {
            return Err(SandboxError::EscapesWorkspace(base.display().to_string()));
        }

        let mut resolved = base;
        for component in missing.into_iter().rev() {
            resolved.push(component);
        }

        // Final containment check using logical path under root
        let rel = resolved
            .strip_prefix(&self.root)
            .map_err(|_| SandboxError::EscapesWorkspace(resolved.display().to_string()))?;
        if rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(SandboxError::EscapesWorkspace(
                resolved.display().to_string(),
            ));
        }

        Ok(resolved)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolves_nested_nonexistent_paths() {
        let dir = std::env::temp_dir().join(format!(
            "desktop-agent-sandbox-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let sandbox = WorkspaceSandbox::new(dir.clone()).unwrap();
        let resolved = sandbox
            .resolve("projects/demo/src/main.rs")
            .unwrap();
        assert_eq!(
            resolved,
            dir.canonicalize().unwrap().join("projects/demo/src/main.rs")
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
