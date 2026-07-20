use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_store::StoreExt;

use crate::agent::{AgentState, AgentStreamEvent, run_agent_turn};
use crate::edits::{ProposedEdit, apply_edit_to_disk};
use crate::llm::types::{ChatMessage, LlmSettings, ProviderKind};
use crate::persistence::{self, Repository, RulesBundle, Session};

const SETTINGS_STORE: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub provider: ProviderKind,
    #[serde(default = "default_preset_id")]
    pub preset_id: String,
    pub model: String,
    pub api_key: String,
    #[serde(default)]
    pub base_url: Option<String>,
}

fn default_preset_id() -> String {
    "ollama".into()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            provider: ProviderKind::Ollama,
            preset_id: default_preset_id(),
            model: "llama3.2".into(),
            api_key: String::new(),
            base_url: Some("http://localhost:11434/v1".into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub message: String,
    pub history: Vec<ChatMessage>,
    pub settings: AppSettings,
}

#[tauri::command]
async fn get_settings(app: AppHandle) -> Result<AppSettings, String> {
    load_settings(&app)
}

#[tauri::command]
async fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let store = app.store(SETTINGS_STORE).map_err(|e| e.to_string())?;
    store.set(
        "settings",
        serde_json::to_value(&settings).map_err(|e| e.to_string())?,
    );
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn set_workspace(state: State<'_, AgentState>, path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }
    *state.workspace.write() = Some(path);
    Ok(())
}

#[tauri::command]
async fn get_workspace(state: State<'_, AgentState>) -> Result<Option<String>, String> {
    Ok(state.workspace.read().clone())
}

#[tauri::command]
async fn send_message(
    app: AppHandle,
    state: State<'_, AgentState>,
    request: SendMessageRequest,
) -> Result<(), String> {
    let settings = LlmSettings {
        provider: request.settings.provider,
        model: request.settings.model,
        api_key: request.settings.api_key,
        base_url: request.settings.base_url,
    };

    let workspace = state.workspace.read().clone();
    let rules = persistence::get_rules_for_workspace(&app, workspace.as_deref())?;

    let app_clone = app.clone();
    let agent_state = (*state).clone();

    tauri::async_runtime::spawn(async move {
        if let Err(err) = run_agent_turn(
            app_clone.clone(),
            agent_state,
            settings,
            request.message,
            request.history,
            rules.global_rules,
            rules.workspace_rules,
        )
        .await
        {
            let _ = app_clone.emit(
                "agent://stream",
                AgentStreamEvent::Error { message: err },
            );
        }
    });

    Ok(())
}

#[tauri::command]
async fn cancel_agent(state: State<'_, AgentState>) -> Result<(), String> {
    state.request_cancel();
    Ok(())
}

#[tauri::command]
async fn list_pending_edits(state: State<'_, AgentState>) -> Result<Vec<ProposedEdit>, String> {
    Ok(state.edits.list())
}

#[tauri::command]
async fn apply_edit(state: State<'_, AgentState>, edit_id: String) -> Result<(), String> {
    let edit = state
        .edits
        .get(&edit_id)
        .ok_or_else(|| format!("Edit not found: {edit_id}"))?;

    let workspace = state
        .workspace
        .read()
        .clone()
        .ok_or_else(|| "No workspace selected".to_string())?;

    apply_edit_to_disk(&PathBuf::from(workspace), &edit)?;
    state.edits.remove(&edit_id);
    Ok(())
}

#[tauri::command]
async fn reject_edit(state: State<'_, AgentState>, edit_id: String) -> Result<(), String> {
    state
        .edits
        .remove(&edit_id)
        .ok_or_else(|| format!("Edit not found: {edit_id}"))?;
    Ok(())
}

#[tauri::command]
async fn get_git_branch(path: Option<String>) -> Result<String, String> {
    let cwd = path
        .filter(|p| !p.trim().is_empty())
        .ok_or_else(|| "No workspace path".to_string())?;

    let output = tokio::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("git failed: {e}"))?;

    if !output.status.success() {
        return Ok("—".into());
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(if branch.is_empty() { "—".into() } else { branch })
}

#[tauri::command]
async fn list_ollama_models(base_url: Option<String>) -> Result<Vec<String>, String> {
    let raw = base_url
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "http://localhost:11434".to_string());
    let root = raw.trim().trim_end_matches('/').trim_end_matches("/v1");
    let url = format!("{root}/api/tags");

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("无法连接 Ollama ({url}): {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Ollama 返回 {status}: {text}"));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析 Ollama 响应失败: {e}"))?;

    let mut names: Vec<String> = value
        .get("models")
        .and_then(|m| m.as_array())
        .map(|models| {
            models
                .iter()
                .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    names.sort();
    if names.is_empty() {
        return Err("Ollama 未返回可用模型，请先运行 ollama pull <model>".into());
    }
    Ok(names)
}

#[tauri::command]
async fn list_repositories(app: AppHandle) -> Result<Vec<Repository>, String> {
    persistence::list_repositories(&app)
}

#[tauri::command]
async fn add_repository(
    app: AppHandle,
    state: State<'_, AgentState>,
    path: String,
) -> Result<Repository, String> {
    let repo = persistence::add_repository(&app, path)?;
    *state.workspace.write() = Some(repo.path.clone());
    Ok(repo)
}

#[tauri::command]
async fn remove_repository(app: AppHandle, id: String) -> Result<(), String> {
    persistence::remove_repository(&app, id)
}

#[tauri::command]
async fn set_active_repository(
    app: AppHandle,
    state: State<'_, AgentState>,
    id: String,
) -> Result<Repository, String> {
    let repo = persistence::set_active_repository(&app, id)?;
    *state.workspace.write() = Some(repo.path.clone());
    Ok(repo)
}

#[tauri::command]
async fn get_active_repository(app: AppHandle) -> Result<Option<Repository>, String> {
    persistence::get_active_repository(&app)
}

#[tauri::command]
async fn list_sessions(app: AppHandle) -> Result<Vec<Session>, String> {
    persistence::list_sessions(&app)
}

#[tauri::command]
async fn create_session(
    app: AppHandle,
    workspace_id: Option<String>,
    title: Option<String>,
) -> Result<Session, String> {
    persistence::create_session(&app, workspace_id, title)
}

#[tauri::command]
async fn update_session(app: AppHandle, session: Session) -> Result<Session, String> {
    persistence::update_session(&app, session)
}

#[tauri::command]
async fn delete_session(app: AppHandle, id: String) -> Result<(), String> {
    persistence::delete_session(&app, id)
}

#[tauri::command]
async fn set_active_session(app: AppHandle, id: String) -> Result<Session, String> {
    persistence::set_active_session(&app, id)
}

#[tauri::command]
async fn get_active_session(app: AppHandle) -> Result<Option<Session>, String> {
    persistence::get_active_session(&app)
}

#[tauri::command]
async fn get_rules(
    app: AppHandle,
    workspace_path: Option<String>,
) -> Result<RulesBundle, String> {
    persistence::get_rules_for_workspace(&app, workspace_path.as_deref())
}

#[tauri::command]
async fn set_global_rules(app: AppHandle, content: String) -> Result<(), String> {
    persistence::set_global_rules(&app, content)
}

#[tauri::command]
async fn set_workspace_rules(workspace_path: String, content: String) -> Result<(), String> {
    persistence::write_workspace_rules(&workspace_path, &content)
}

fn load_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let store = app.store(SETTINGS_STORE).map_err(|e| e.to_string())?;
    match store.get("settings") {
        Some(value) => serde_json::from_value(value).map_err(|e| e.to_string()),
        None => Ok(AppSettings::default()),
    }
}

pub fn register(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.invoke_handler(tauri::generate_handler![
        get_settings,
        save_settings,
        set_workspace,
        get_workspace,
        send_message,
        cancel_agent,
        list_pending_edits,
        apply_edit,
        reject_edit,
        list_ollama_models,
        get_git_branch,
        list_repositories,
        add_repository,
        remove_repository,
        set_active_repository,
        get_active_repository,
        list_sessions,
        create_session,
        update_session,
        delete_session,
        set_active_session,
        get_active_session,
        get_rules,
        set_global_rules,
        set_workspace_rules,
    ])
}
