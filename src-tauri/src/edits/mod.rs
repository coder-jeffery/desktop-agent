use std::collections::HashMap;
use std::fs;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedEdit {
    pub id: String,
    pub path: String,
    pub old_content: String,
    pub new_content: String,
    pub diff: String,
    pub summary: String,
}

impl ProposedEdit {
    pub fn new(path: String, old_content: String, new_content: String) -> Self {
        let diff = unified_diff(&old_content, &new_content, &path);
        let summary = summarize_diff(&old_content, &new_content);
        Self {
            id: Uuid::new_v4().to_string(),
            path,
            old_content,
            new_content,
            diff,
            summary,
        }
    }
}

#[derive(Default)]
pub struct EditStore {
    edits: RwLock<HashMap<String, ProposedEdit>>,
}

impl EditStore {
    pub fn insert(&self, edit: ProposedEdit) {
        self.edits.write().insert(edit.id.clone(), edit);
    }

    pub fn get(&self, id: &str) -> Option<ProposedEdit> {
        self.edits.read().get(id).cloned()
    }

    pub fn list(&self) -> Vec<ProposedEdit> {
        self.edits.read().values().cloned().collect()
    }

    pub fn remove(&self, id: &str) -> Option<ProposedEdit> {
        self.edits.write().remove(id)
    }
}

pub fn unified_diff(old: &str, new: &str, path: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut out = String::new();
    out.push_str(&format!("--- a/{path}\n"));
    out.push_str(&format!("+++ b/{path}\n"));
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        out.push_str(sign);
        out.push_str(change.value());
        if !change.value().ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

fn summarize_diff(old: &str, new: &str) -> String {
    let old_lines = old.lines().count();
    let new_lines = new.lines().count();
    if old.is_empty() {
        format!("Create file ({new_lines} lines)")
    } else if new.is_empty() {
        format!("Delete file ({old_lines} lines)")
    } else {
        format!("Modify file ({old_lines} -> {new_lines} lines)")
    }
}

pub fn apply_edit_to_disk(
    workspace_root: &std::path::Path,
    edit: &ProposedEdit,
) -> Result<(), String> {
    let path = workspace_root.join(&edit.path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, &edit.new_content).map_err(|e| e.to_string())?;
    Ok(())
}
