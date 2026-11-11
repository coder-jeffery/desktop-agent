use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

/// Default agent workspace: sibling folder next to the app project (dev), or under Documents (release).
pub fn default_workspace_dir(app: &AppHandle) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let app_root = manifest
            .parent()
            .ok_or_else(|| "cannot resolve app root".to_string())?;
        let parent = app_root
            .parent()
            .ok_or_else(|| "cannot resolve parent directory".to_string())?;
        let stem = app_root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("desktop-agent");
        return Ok(parent.join(format!("{stem}-workspace")));
    }

    let docs = app
        .path()
        .document_dir()
        .map_err(|e| format!("document dir: {e}"))?;
    Ok(docs.join("Desktop Agent").join("workspace"))
}

/// Create the default workspace directory tree if missing.
pub fn ensure_default_workspace_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let path = default_workspace_dir(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path.canonicalize().unwrap_or(path))
}
