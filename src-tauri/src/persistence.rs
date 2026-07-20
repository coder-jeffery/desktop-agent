use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

const STORE_PATH: &str = "app_data.json";

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn repo_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub id: String,
    pub path: String,
    pub name: String,
    pub last_opened: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredMessage {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<StoredToolCall>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub title: String,
    pub workspace_id: Option<String>,
    pub messages: Vec<StoredMessage>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    #[serde(default)]
    pub repositories: Vec<Repository>,
    #[serde(default)]
    pub sessions: Vec<Session>,
    #[serde(default)]
    pub active_session_id: Option<String>,
    #[serde(default)]
    pub active_repository_id: Option<String>,
    #[serde(default)]
    pub global_rules: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesBundle {
    pub global_rules: String,
    pub workspace_rules: String,
}

fn load_data(app: &AppHandle) -> Result<AppData, String> {
    let store = app.store(STORE_PATH).map_err(|e| e.to_string())?;
    match store.get("data") {
        Some(value) => serde_json::from_value(value).map_err(|e| e.to_string()),
        None => Ok(AppData::default()),
    }
}

fn save_data(app: &AppHandle, data: &AppData) -> Result<(), String> {
    let store = app.store(STORE_PATH).map_err(|e| e.to_string())?;
    store.set(
        "data",
        serde_json::to_value(data).map_err(|e| e.to_string())?,
    );
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

fn workspace_rules_path(workspace: &str) -> PathBuf {
    PathBuf::from(workspace).join(".desktop-agent").join("rules.md")
}

pub fn read_workspace_rules(workspace: &str) -> String {
    let path = workspace_rules_path(workspace);
    fs::read_to_string(path).unwrap_or_default()
}

pub fn write_workspace_rules(workspace: &str, content: &str) -> Result<(), String> {
    let path = workspace_rules_path(workspace);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, content).map_err(|e| e.to_string())
}

pub fn get_rules_for_workspace(app: &AppHandle, workspace: Option<&str>) -> Result<RulesBundle, String> {
    let data = load_data(app)?;
    let workspace_rules = workspace
        .map(read_workspace_rules)
        .unwrap_or_default();
    Ok(RulesBundle {
        global_rules: data.global_rules,
        workspace_rules,
    })
}

pub fn list_repositories(app: &AppHandle) -> Result<Vec<Repository>, String> {
    let mut data = load_data(app)?;
    data.repositories
        .sort_by(|a, b| b.last_opened.cmp(&a.last_opened));
    Ok(data.repositories)
}

pub fn add_repository(app: &AppHandle, path: String) -> Result<Repository, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }
    let canonical = path_buf
        .canonicalize()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();

    let mut data = load_data(app)?;
    if let Some(existing) = data
        .repositories
        .iter_mut()
        .find(|r| r.path == canonical)
    {
        existing.last_opened = now_ms();
        let repo = existing.clone();
        data.active_repository_id = Some(repo.id.clone());
        save_data(app, &data)?;
        return Ok(repo);
    }

    let repo = Repository {
        id: Uuid::new_v4().to_string(),
        name: repo_name_from_path(&canonical),
        path: canonical,
        last_opened: now_ms(),
    };
    data.active_repository_id = Some(repo.id.clone());
    data.repositories.push(repo.clone());
    save_data(app, &data)?;
    Ok(repo)
}

pub fn remove_repository(app: &AppHandle, id: String) -> Result<(), String> {
    let mut data = load_data(app)?;
    data.repositories.retain(|r| r.id != id);
    if data.active_repository_id.as_deref() == Some(&id) {
        data.active_repository_id = data.repositories.first().map(|r| r.id.clone());
    }
    for session in &mut data.sessions {
        if session.workspace_id.as_deref() == Some(&id) {
            session.workspace_id = None;
        }
    }
    save_data(app, &data)?;
    Ok(())
}

pub fn set_active_repository(app: &AppHandle, id: String) -> Result<Repository, String> {
    let mut data = load_data(app)?;
    let repo = data
        .repositories
        .iter_mut()
        .find(|r| r.id == id)
        .ok_or_else(|| format!("Repository not found: {id}"))?;
    repo.last_opened = now_ms();
    let cloned = repo.clone();
    data.active_repository_id = Some(id);
    save_data(app, &data)?;
    Ok(cloned)
}

pub fn get_active_repository(app: &AppHandle) -> Result<Option<Repository>, String> {
    let data = load_data(app)?;
    Ok(data
        .active_repository_id
        .as_ref()
        .and_then(|id| data.repositories.iter().find(|r| &r.id == id))
        .cloned())
}

pub fn list_sessions(app: &AppHandle) -> Result<Vec<Session>, String> {
    let mut data = load_data(app)?;
    data.sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(data.sessions)
}

pub fn create_session(
    app: &AppHandle,
    workspace_id: Option<String>,
    title: Option<String>,
) -> Result<Session, String> {
    let mut data = load_data(app)?;
    let now = now_ms();
    let session = Session {
        id: Uuid::new_v4().to_string(),
        title: title.unwrap_or_else(|| "New Agent".into()),
        workspace_id,
        messages: Vec::new(),
        created_at: now,
        updated_at: now,
    };
    data.active_session_id = Some(session.id.clone());
    data.sessions.insert(0, session.clone());
    save_data(app, &data)?;
    Ok(session)
}

pub fn update_session(app: &AppHandle, session: Session) -> Result<Session, String> {
    let mut data = load_data(app)?;
    let mut updated = session;
    updated.updated_at = now_ms();
    if let Some(existing) = data.sessions.iter_mut().find(|s| s.id == updated.id) {
        *existing = updated.clone();
    } else {
        data.sessions.insert(0, updated.clone());
    }
    save_data(app, &data)?;
    Ok(updated)
}

pub fn delete_session(app: &AppHandle, id: String) -> Result<(), String> {
    let mut data = load_data(app)?;
    data.sessions.retain(|s| s.id != id);
    if data.active_session_id.as_deref() == Some(&id) {
        data.active_session_id = data.sessions.first().map(|s| s.id.clone());
    }
    save_data(app, &data)?;
    Ok(())
}

pub fn set_active_session(app: &AppHandle, id: String) -> Result<Session, String> {
    let mut data = load_data(app)?;
    let session = data
        .sessions
        .iter()
        .find(|s| s.id == id)
        .cloned()
        .ok_or_else(|| format!("Session not found: {id}"))?;
    data.active_session_id = Some(id);
    save_data(app, &data)?;
    Ok(session)
}

pub fn get_active_session(app: &AppHandle) -> Result<Option<Session>, String> {
    let data = load_data(app)?;
    Ok(data
        .active_session_id
        .as_ref()
        .and_then(|id| data.sessions.iter().find(|s| &s.id == id))
        .cloned())
}

pub fn set_global_rules(app: &AppHandle, content: String) -> Result<(), String> {
    let mut data = load_data(app)?;
    data.global_rules = content;
    save_data(app, &data)
}
