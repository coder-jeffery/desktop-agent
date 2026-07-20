use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures::StreamExt;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::edits::{EditStore, ProposedEdit};
use crate::llm::{self, types::*};
use crate::tools::{ToolContext, execute_tool, sandbox_from_path, tool_definitions};

const MAX_TOOL_ITERATIONS: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentStreamEvent {
    TextDelta { delta: String },
    ToolCall { id: String, name: String, arguments: String },
    ToolResult { id: String, name: String, output: String },
    ProposedEdit { edit: ProposedEdit },
    TurnComplete,
    Error { message: String },
}

#[derive(Clone)]
pub struct AgentState {
    pub cancel: Arc<AtomicBool>,
    pub workspace: Arc<RwLock<Option<String>>>,
    pub edits: Arc<EditStore>,
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
            workspace: Arc::new(RwLock::new(None)),
            edits: Arc::new(EditStore::default()),
        }
    }

    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn reset_cancel(&self) {
        self.cancel.store(false, Ordering::SeqCst);
    }
}

fn system_prompt(workspace: &str, global_rules: &str, workspace_rules: &str) -> String {
    let mut prompt = format!(
        r#"You are Desktop Agent, a local coding assistant running on the user's machine.

Workspace root: {workspace}

You can read files, search code, propose file edits, run shell commands, and perform basic git operations.
When modifying files, use write_file to propose changes. The user must approve edits before they are applied.
Prefer small, focused changes. Read relevant files before editing.
When running shell commands, explain what you are doing briefly.
If a tool fails, diagnose and try a different approach."#
    );

    let global = global_rules.trim();
    if !global.is_empty() {
        prompt.push_str("\n\n# Global Rules\n");
        prompt.push_str(global);
    }

    let local = workspace_rules.trim();
    if !local.is_empty() {
        prompt.push_str("\n\n# Workspace Rules\n");
        prompt.push_str(local);
    }

    prompt
}

pub async fn run_agent_turn(
    app: AppHandle,
    state: AgentState,
    settings: LlmSettings,
    user_message: String,
    history: Vec<ChatMessage>,
    global_rules: String,
    workspace_rules: String,
) -> Result<(), String> {
    state.reset_cancel();

    let workspace = state
        .workspace
        .read()
        .clone()
        .ok_or_else(|| "No workspace selected".to_string())?;

    let sandbox = sandbox_from_path(&workspace).map_err(|e| e.to_string())?;
    let tools = tool_definitions();
    let mut messages = history;
    messages.push(ChatMessage {
        role: MessageRole::User,
        content: user_message,
        tool_calls: None,
        tool_call_id: None,
        name: None,
    });

    let ctx = ToolContext {
        sandbox: &sandbox,
        edits: &state.edits,
    };

    let system = system_prompt(&workspace, &global_rules, &workspace_rules);

    for _ in 0..MAX_TOOL_ITERATIONS {
        if state.cancel.load(Ordering::SeqCst) {
            emit(&app, AgentStreamEvent::Error {
                message: "Cancelled".into(),
            })?;
            return Ok(());
        }

        let req = ChatRequest {
            settings: settings.clone(),
            system: system.clone(),
            messages: messages.clone(),
            tools: tools.clone(),
        };

        let mut stream = llm::stream_chat(req).await?;
        let mut assistant_text = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();

        while let Some(event) = stream.next().await {
            if state.cancel.load(Ordering::SeqCst) {
                emit(&app, AgentStreamEvent::Error {
                    message: "Cancelled".into(),
                })?;
                return Ok(());
            }

            match event {
                Ok(StreamEvent::TextDelta(delta)) => {
                    if !delta.is_empty() {
                        assistant_text.push_str(&delta);
                        emit(
                            &app,
                            AgentStreamEvent::TextDelta { delta },
                        )?;
                    }
                }
                Ok(StreamEvent::ToolCalls(calls)) => {
                    tool_calls = calls;
                }
                Ok(StreamEvent::Done) => break,
                Err(err) => {
                    emit(&app, AgentStreamEvent::Error { message: err })?;
                    return Ok(());
                }
            }
        }

        if tool_calls.is_empty() {
            messages.push(ChatMessage {
                role: MessageRole::Assistant,
                content: assistant_text,
                tool_calls: None,
                tool_call_id: None,
                name: None,
            });
            emit(&app, AgentStreamEvent::TurnComplete)?;
            return Ok(());
        }

        messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: assistant_text,
            tool_calls: Some(tool_calls.clone()),
            tool_call_id: None,
            name: None,
        });

        for tc in tool_calls {
            emit(
                &app,
                AgentStreamEvent::ToolCall {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    arguments: tc.arguments.clone(),
                },
            )?;

            let result = execute_tool(&ctx, &tc.name, &tc.arguments).await;

            if tc.name == "write_file" {
                if let Ok(args) = serde_json::from_str::<serde_json::Value>(&tc.arguments) {
                    if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                        if let Some(edit) = state
                            .edits
                            .list()
                            .into_iter()
                            .rev()
                            .find(|e| e.path == path || e.path.ends_with(path))
                        {
                            emit(
                                &app,
                                AgentStreamEvent::ProposedEdit {
                                    edit: edit.clone(),
                                },
                            )?;
                        }
                    }
                }
            }

            let output = match result {
                Ok(out) => out,
                Err(err) => format!("Error: {err}"),
            };

            emit(
                &app,
                AgentStreamEvent::ToolResult {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    output: output.clone(),
                },
            )?;

            messages.push(ChatMessage {
                role: MessageRole::Tool,
                content: output,
                tool_calls: None,
                tool_call_id: Some(tc.id),
                name: Some(tc.name),
            });
        }
    }

    emit(
        &app,
        AgentStreamEvent::Error {
            message: "Maximum tool iterations reached".into(),
        },
    )?;
    Ok(())
}

fn emit(app: &AppHandle, event: AgentStreamEvent) -> Result<(), String> {
    app.emit("agent://stream", event)
        .map_err(|e| format!("emit failed: {e}"))
}
