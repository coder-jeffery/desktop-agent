use std::fs;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;
use regex::Regex;
use serde_json::{Value, json};
use tokio::process::Command;
use tokio::time::{Duration, timeout};
use walkdir::WalkDir;

mod sandbox;
mod init_project;

use sandbox::{WorkspaceSandbox, is_shell_command_blocked};
use crate::edits::{EditStore, ProposedEdit};
use init_project::init_project;

const MAX_READ_BYTES: u64 = 256 * 1024;

pub struct ToolContext<'a> {
    pub sandbox: &'a WorkspaceSandbox,
    pub edits: &'a EditStore,
}

pub fn tool_definitions() -> Vec<crate::llm::types::ToolDefinition> {
    vec![
        tool(
            "read_file",
            "Read a text file from the workspace",
            json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Relative path from workspace root"}
                },
                "required": ["path"]
            }),
        ),
        tool(
            "write_file",
            "Propose writing or replacing a file. Changes require user approval before being applied. Prefer idiomatic language style and place files inside the initialized project directory.",
            json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Relative path from workspace root, e.g. my-app/src/main.rs"},
                    "content": {"type": "string", "description": "Full new file content"}
                },
                "required": ["path", "content"]
            }),
        ),
        tool(
            "init_project",
            "Initialize a new idiomatic programming-language project under the workspace (scaffold config + entry files). Use this BEFORE writing feature code when the user asks to generate an app/project.",
            json!({
                "type": "object",
                "properties": {
                    "language": {
                        "type": "string",
                        "description": "Programming language: rust, typescript, javascript, python, go, java, csharp, cpp, php, ruby"
                    },
                    "name": {
                        "type": "string",
                        "description": "Project folder/package name, e.g. hello-api"
                    },
                    "directory": {
                        "type": "string",
                        "description": "Parent directory relative to workspace root. Default '.' (workspace root)."
                    }
                },
                "required": ["language", "name"]
            }),
        ),
        tool(
            "list_dir",
            "List files and directories under a workspace path",
            json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Relative directory path, default workspace root"}
                },
                "required": []
            }),
        ),
        tool(
            "glob",
            "Find files matching a glob pattern relative to workspace root",
            json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Glob like **/*.rs"}
                },
                "required": ["pattern"]
            }),
        ),
        tool(
            "grep",
            "Search file contents in the workspace using a regex pattern",
            json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string"},
                    "glob": {"type": "string", "description": "Optional glob filter, default **/*"}
                },
                "required": ["pattern"]
            }),
        ),
        tool(
            "run_shell",
            "Run a shell command in the workspace directory. Use for tests, builds, etc.",
            json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string"}
                },
                "required": ["command"]
            }),
        ),
        tool(
            "git_status",
            "Show git status for the workspace",
            json!({"type": "object", "properties": {}, "required": []}),
        ),
        tool(
            "git_diff",
            "Show git diff for the workspace",
            json!({
                "type": "object",
                "properties": {
                    "staged": {"type": "boolean", "description": "Show staged diff only"}
                },
                "required": []
            }),
        ),
        tool(
            "git_commit",
            "Create a git commit with all staged changes",
            json!({
                "type": "object",
                "properties": {
                    "message": {"type": "string"}
                },
                "required": ["message"]
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, schema: Value) -> crate::llm::types::ToolDefinition {
    crate::llm::types::ToolDefinition {
        name: name.to_string(),
        description: description.to_string(),
        input_schema: schema,
    }
}

pub async fn execute_tool(
    ctx: &ToolContext<'_>,
    name: &str,
    arguments: &str,
) -> Result<String, String> {
    let args: Value =
        serde_json::from_str(arguments).unwrap_or_else(|_| json!({}));

    match name {
        "read_file" => read_file(ctx, &args),
        "write_file" => write_file(ctx, &args),
        "init_project" => init_project(ctx, &args),
        "list_dir" => list_dir(ctx, &args),
        "glob" => glob_search(ctx, &args),
        "grep" => grep_search(ctx, &args),
        "run_shell" => run_shell(ctx, &args).await,
        "git_status" => git_status(ctx).await,
        "git_diff" => git_diff(ctx, &args).await,
        "git_commit" => git_commit(ctx, &args).await,
        other => Err(format!("unknown tool: {other}")),
    }
}

fn read_file(ctx: &ToolContext<'_>, args: &Value) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or_else(|| "missing path".to_string())?;
    let resolved = ctx.sandbox.resolve(path).map_err(|e| e.to_string())?;
    if !resolved.is_file() {
        return Err(format!("not a file: {path}"));
    }
    let meta = fs::metadata(&resolved).map_err(|e| e.to_string())?;
    if meta.len() > MAX_READ_BYTES {
        return Err(format!(
            "file too large ({} bytes, max {MAX_READ_BYTES})",
            meta.len()
        ));
    }
    let content = fs::read_to_string(&resolved).map_err(|e| e.to_string())?;
    Ok(content)
}

fn write_file(ctx: &ToolContext<'_>, args: &Value) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or_else(|| "missing path".to_string())?;
    let content = args
        .get("content")
        .and_then(|c| c.as_str())
        .ok_or_else(|| "missing content".to_string())?;

    if content.len() as u64 > MAX_READ_BYTES {
        return Err(format!(
            "content too large ({} bytes, max {MAX_READ_BYTES})",
            content.len()
        ));
    }

    let resolved = ctx.sandbox.resolve(path).map_err(|e| e.to_string())?;
    let relative = ctx.sandbox.relative_display(&resolved);
    let old_content = if resolved.is_file() {
        fs::read_to_string(&resolved).map_err(|e| e.to_string())?
    } else {
        String::new()
    };

    let edit = ProposedEdit::new(relative, old_content, content.to_string());
    let edit_id = edit.id.clone();
    let summary = edit.summary.clone();
    ctx.edits.insert(edit);

    Ok(format!(
        "Proposed edit {edit_id} for {path}. {summary}. Waiting for user approval."
    ))
}

fn list_dir(ctx: &ToolContext<'_>, args: &Value) -> Result<String, String> {
    let path = args.get("path").and_then(|p| p.as_str()).unwrap_or(".");
    let resolved = ctx.sandbox.resolve(path).map_err(|e| e.to_string())?;
    if !resolved.is_dir() {
        return Err(format!("not a directory: {path}"));
    }

    let mut entries: Vec<String> = fs::read_dir(&resolved)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if e.path().is_dir() {
                format!("{name}/")
            } else {
                name
            }
        })
        .collect();
    entries.sort();
    Ok(entries.join("\n"))
}

fn glob_search(ctx: &ToolContext<'_>, args: &Value) -> Result<String, String> {
    let pattern = args
        .get("pattern")
        .and_then(|p| p.as_str())
        .ok_or_else(|| "missing pattern".to_string())?;

    let mut builder = GlobSetBuilder::new();
    builder.add(Glob::new(pattern).map_err(|e| e.to_string())?);
    let set = builder.build().map_err(|e| e.to_string())?;

    let mut matches = Vec::new();
    for entry in WalkDir::new(ctx.sandbox.root())
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            let rel = ctx.sandbox.relative_display(path);
            if set.is_match(&rel) {
                matches.push(rel);
            }
        }
    }
    matches.sort();
    matches.truncate(200);
    Ok(matches.join("\n"))
}

fn grep_search(ctx: &ToolContext<'_>, args: &Value) -> Result<String, String> {
    let pattern = args
        .get("pattern")
        .and_then(|p| p.as_str())
        .ok_or_else(|| "missing pattern".to_string())?;
    let glob_filter = args.get("glob").and_then(|g| g.as_str()).unwrap_or("**/*");

    let re = Regex::new(pattern).map_err(|e| format!("invalid regex: {e}"))?;
    let mut glob_builder = GlobSetBuilder::new();
    glob_builder.add(Glob::new(glob_filter).map_err(|e| e.to_string())?);
    let glob_set = glob_builder.build().map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    let walker = WalkBuilder::new(ctx.sandbox.root())
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true)
        .build();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let rel = ctx.sandbox.relative_display(path);
        if !glob_set.is_match(&rel) {
            continue;
        }
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        for (idx, line) in content.lines().enumerate() {
            if re.is_match(line) {
                results.push(format!("{}:{}:{}", rel, idx + 1, line));
                if results.len() >= 100 {
                    return Ok(results.join("\n"));
                }
            }
        }
    }

    Ok(if results.is_empty() {
        "No matches".into()
    } else {
        results.join("\n")
    })
}

async fn run_shell(ctx: &ToolContext<'_>, args: &Value) -> Result<String, String> {
    let command = args
        .get("command")
        .and_then(|c| c.as_str())
        .ok_or_else(|| "missing command".to_string())?;

    if let Some(reason) = is_shell_command_blocked(command) {
        return Err(format!("blocked command: {reason}"));
    }

    let output = timeout(Duration::from_secs(30), Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(ctx.sandbox.root())
        .output())
        .await
        .map_err(|_| "command timed out after 30 seconds".to_string())?
        .map_err(|e| format!("failed to run command: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let status = output.status.code().unwrap_or(-1);

    Ok(format!(
        "exit_code: {status}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    ))
}

async fn git_status(ctx: &ToolContext<'_>) -> Result<String, String> {
    run_git(ctx.sandbox.root(), &["status", "--short"]).await
}

async fn git_diff(ctx: &ToolContext<'_>, args: &Value) -> Result<String, String> {
    let staged = args.get("staged").and_then(|s| s.as_bool()).unwrap_or(false);
    if staged {
        run_git(ctx.sandbox.root(), &["diff", "--staged"]).await
    } else {
        run_git(ctx.sandbox.root(), &["diff"]).await
    }
}

async fn git_commit(ctx: &ToolContext<'_>, args: &Value) -> Result<String, String> {
    let message = args
        .get("message")
        .and_then(|m| m.as_str())
        .ok_or_else(|| "missing commit message".to_string())?;
    if message.trim().is_empty() {
        return Err("commit message cannot be empty".into());
    }

    run_git(
        ctx.sandbox.root(),
        &["commit", "-m", message],
    )
    .await
}

async fn run_git(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .await
        .map_err(|e| format!("git failed: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let status = output.status.code().unwrap_or(-1);

    if status != 0 {
        return Err(format!("git exited with {status}: {stderr}{stdout}"));
    }

    Ok(if stdout.is_empty() {
        stderr.to_string()
    } else {
        stdout.to_string()
    })
}

pub fn sandbox_from_path(path: &str) -> Result<WorkspaceSandbox, sandbox::SandboxError> {
    WorkspaceSandbox::new(PathBuf::from(path))
}
