use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

const DB_FILE: &str = "desktop_agent.db";
const LEGACY_STORE: &str = "app_data.json";

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
struct LegacyAppData {
    #[serde(default)]
    repositories: Vec<Repository>,
    #[serde(default)]
    sessions: Vec<Session>,
    #[serde(default)]
    active_session_id: Option<String>,
    #[serde(default)]
    active_repository_id: Option<String>,
    #[serde(default)]
    global_rules: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesBundle {
    pub global_rules: String,
    pub workspace_rules: String,
}

pub struct DbState(pub Mutex<Connection>);

fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(DB_FILE))
}

fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;

        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS repositories (
            id TEXT PRIMARY KEY NOT NULL,
            path TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            last_opened INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY NOT NULL,
            title TEXT NOT NULL,
            workspace_id TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            seq INTEGER NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            tool_calls TEXT,
            tool_call_id TEXT,
            name TEXT,
            UNIQUE(session_id, seq),
            FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_messages_session_seq
            ON messages(session_id, seq);
        CREATE INDEX IF NOT EXISTS idx_sessions_updated
            ON sessions(updated_at DESC);
        "#,
    )
    .map_err(|e| e.to_string())
}

fn meta_get(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM meta WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn meta_set(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn meta_delete(conn: &Connection, key: &str) -> Result<(), String> {
    conn.execute("DELETE FROM meta WHERE key = ?1", params![key])
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn load_messages(conn: &Connection, session_id: &str) -> Result<Vec<StoredMessage>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT role, content, tool_calls, tool_call_id, name
             FROM messages WHERE session_id = ?1 ORDER BY seq ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![session_id], |row| {
            let tool_calls_json: Option<String> = row.get(2)?;
            let tool_calls = tool_calls_json
                .as_deref()
                .filter(|s| !s.is_empty())
                .and_then(|s| serde_json::from_str(s).ok());
            Ok(StoredMessage {
                role: row.get(0)?,
                content: row.get(1)?,
                tool_calls,
                tool_call_id: row.get(3)?,
                name: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut messages = Vec::new();
    for row in rows {
        messages.push(row.map_err(|e| e.to_string())?);
    }
    Ok(messages)
}

fn replace_messages(
    conn: &Connection,
    session_id: &str,
    messages: &[StoredMessage],
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM messages WHERE session_id = ?1",
        params![session_id],
    )
    .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "INSERT INTO messages(session_id, seq, role, content, tool_calls, tool_call_id, name)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .map_err(|e| e.to_string())?;

    for (seq, msg) in messages.iter().enumerate() {
        let tool_calls = msg
            .tool_calls
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        stmt.execute(params![
            session_id,
            seq as i64,
            msg.role,
            msg.content,
            tool_calls,
            msg.tool_call_id,
            msg.name,
        ])
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn load_session(conn: &Connection, id: &str) -> Result<Option<Session>, String> {
    let row = conn
        .query_row(
            "SELECT id, title, workspace_id, created_at, updated_at
             FROM sessions WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let Some((id, title, workspace_id, created_at, updated_at)) = row else {
        return Ok(None);
    };

    Ok(Some(Session {
        messages: load_messages(conn, &id)?,
        id,
        title,
        workspace_id,
        created_at,
        updated_at,
    }))
}

fn migrate_from_json_store(app: &AppHandle, conn: &Connection) -> Result<(), String> {
    if meta_get(conn, "migrated_from_json")?.as_deref() == Some("1") {
        return Ok(());
    }

    // Skip if we already have sessions in SQLite
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if count > 0 {
        meta_set(conn, "migrated_from_json", "1")?;
        return Ok(());
    }

    let legacy = match app.store(LEGACY_STORE) {
        Ok(store) => match store.get("data") {
            Some(value) => serde_json::from_value::<LegacyAppData>(value).unwrap_or_default(),
            None => {
                meta_set(conn, "migrated_from_json", "1")?;
                return Ok(());
            }
        },
        Err(_) => {
            meta_set(conn, "migrated_from_json", "1")?;
            return Ok(());
        }
    };

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    for repo in &legacy.repositories {
        tx.execute(
            "INSERT OR REPLACE INTO repositories(id, path, name, last_opened)
             VALUES(?1, ?2, ?3, ?4)",
            params![repo.id, repo.path, repo.name, repo.last_opened],
        )
        .map_err(|e| e.to_string())?;
    }

    for session in &legacy.sessions {
        tx.execute(
            "INSERT OR REPLACE INTO sessions(id, title, workspace_id, created_at, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5)",
            params![
                session.id,
                session.title,
                session.workspace_id,
                session.created_at,
                session.updated_at
            ],
        )
        .map_err(|e| e.to_string())?;
        replace_messages(&tx, &session.id, &session.messages)?;
    }

    if let Some(id) = &legacy.active_session_id {
        meta_set(&tx, "active_session_id", id)?;
    }
    if let Some(id) = &legacy.active_repository_id {
        meta_set(&tx, "active_repository_id", id)?;
    }
    if !legacy.global_rules.is_empty() {
        meta_set(&tx, "global_rules", &legacy.global_rules)?;
    }

    meta_set(&tx, "migrated_from_json", "1")?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn init_db(app: &AppHandle) -> Result<DbState, String> {
    let path = db_path(app)?;
    let conn = Connection::open(&path).map_err(|e| format!("open sqlite: {e}"))?;
    init_schema(&conn)?;
    migrate_from_json_store(app, &conn)?;
    Ok(DbState(Mutex::new(conn)))
}

fn with_db<T>(app: &AppHandle, f: impl FnOnce(&Connection) -> Result<T, String>) -> Result<T, String> {
    let state = app.state::<DbState>();
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    f(&conn)
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
    let global_rules = with_db(app, |conn| {
        Ok(meta_get(conn, "global_rules")?.unwrap_or_default())
    })?;
    let workspace_rules = workspace
        .map(read_workspace_rules)
        .unwrap_or_default();
    Ok(RulesBundle {
        global_rules,
        workspace_rules,
    })
}

pub fn list_repositories(app: &AppHandle) -> Result<Vec<Repository>, String> {
    with_db(app, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, path, name, last_opened FROM repositories
                 ORDER BY last_opened DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Repository {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    name: row.get(2)?,
                    last_opened: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
        Ok(out)
    })
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

    with_db(app, |conn| {
        if let Some(existing) = conn
            .query_row(
                "SELECT id, path, name, last_opened FROM repositories WHERE path = ?1",
                params![canonical],
                |row| {
                    Ok(Repository {
                        id: row.get(0)?,
                        path: row.get(1)?,
                        name: row.get(2)?,
                        last_opened: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())?
        {
            let now = now_ms();
            conn.execute(
                "UPDATE repositories SET last_opened = ?1 WHERE id = ?2",
                params![now, existing.id],
            )
            .map_err(|e| e.to_string())?;
            meta_set(conn, "active_repository_id", &existing.id)?;
            return Ok(Repository {
                last_opened: now,
                ..existing
            });
        }

        let repo = Repository {
            id: Uuid::new_v4().to_string(),
            name: repo_name_from_path(&canonical),
            path: canonical,
            last_opened: now_ms(),
        };
        conn.execute(
            "INSERT INTO repositories(id, path, name, last_opened) VALUES(?1, ?2, ?3, ?4)",
            params![repo.id, repo.path, repo.name, repo.last_opened],
        )
        .map_err(|e| e.to_string())?;
        meta_set(conn, "active_repository_id", &repo.id)?;
        Ok(repo)
    })
}

pub fn remove_repository(app: &AppHandle, id: String) -> Result<(), String> {
    with_db(app, |conn| {
        conn.execute("DELETE FROM repositories WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE sessions SET workspace_id = NULL WHERE workspace_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;

        if meta_get(conn, "active_repository_id")?.as_deref() == Some(&id) {
            let next: Option<String> = conn
                .query_row(
                    "SELECT id FROM repositories ORDER BY last_opened DESC LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            match next {
                Some(nid) => meta_set(conn, "active_repository_id", &nid)?,
                None => meta_delete(conn, "active_repository_id")?,
            }
        }
        Ok(())
    })
}

pub fn set_active_repository(app: &AppHandle, id: String) -> Result<Repository, String> {
    with_db(app, |conn| {
        let now = now_ms();
        let changed = conn
            .execute(
                "UPDATE repositories SET last_opened = ?1 WHERE id = ?2",
                params![now, id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err(format!("Repository not found: {id}"));
        }
        meta_set(conn, "active_repository_id", &id)?;
        conn.query_row(
            "SELECT id, path, name, last_opened FROM repositories WHERE id = ?1",
            params![id],
            |row| {
                Ok(Repository {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    name: row.get(2)?,
                    last_opened: row.get(3)?,
                })
            },
        )
        .map_err(|e| e.to_string())
    })
}

pub fn get_active_repository(app: &AppHandle) -> Result<Option<Repository>, String> {
    with_db(app, |conn| {
        let Some(id) = meta_get(conn, "active_repository_id")? else {
            return Ok(None);
        };
        conn.query_row(
            "SELECT id, path, name, last_opened FROM repositories WHERE id = ?1",
            params![id],
            |row| {
                Ok(Repository {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    name: row.get(2)?,
                    last_opened: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())
    })
}

pub fn list_sessions(app: &AppHandle) -> Result<Vec<Session>, String> {
    with_db(app, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, title, workspace_id, created_at, updated_at
                 FROM sessions ORDER BY updated_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut sessions = Vec::new();
        for row in rows {
            let (id, title, workspace_id, created_at, updated_at) =
                row.map_err(|e| e.to_string())?;
            let messages = load_messages(conn, &id)?;
            sessions.push(Session {
                id,
                title,
                workspace_id,
                messages,
                created_at,
                updated_at,
            });
        }
        Ok(sessions)
    })
}

pub fn create_session(
    app: &AppHandle,
    workspace_id: Option<String>,
    title: Option<String>,
) -> Result<Session, String> {
    with_db(app, |conn| {
        let now = now_ms();
        let session = Session {
            id: Uuid::new_v4().to_string(),
            title: title.unwrap_or_else(|| "New Agent".into()),
            workspace_id,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        conn.execute(
            "INSERT INTO sessions(id, title, workspace_id, created_at, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5)",
            params![
                session.id,
                session.title,
                session.workspace_id,
                session.created_at,
                session.updated_at
            ],
        )
        .map_err(|e| e.to_string())?;
        meta_set(conn, "active_session_id", &session.id)?;
        Ok(session)
    })
}

pub fn update_session(app: &AppHandle, session: Session) -> Result<Session, String> {
    with_db(app, |conn| {
        let mut updated = session;
        updated.updated_at = now_ms();

        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM sessions WHERE id = ?1",
                params![updated.id],
                |_| Ok(true),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or(false);

        if exists {
            conn.execute(
                "UPDATE sessions SET title = ?1, workspace_id = ?2, updated_at = ?3 WHERE id = ?4",
                params![
                    updated.title,
                    updated.workspace_id,
                    updated.updated_at,
                    updated.id
                ],
            )
            .map_err(|e| e.to_string())?;
        } else {
            conn.execute(
                "INSERT INTO sessions(id, title, workspace_id, created_at, updated_at)
                 VALUES(?1, ?2, ?3, ?4, ?5)",
                params![
                    updated.id,
                    updated.title,
                    updated.workspace_id,
                    updated.created_at,
                    updated.updated_at
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        replace_messages(conn, &updated.id, &updated.messages)?;
        Ok(updated)
    })
}

pub fn delete_session(app: &AppHandle, id: String) -> Result<(), String> {
    with_db(app, |conn| {
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        if meta_get(conn, "active_session_id")?.as_deref() == Some(&id) {
            let next: Option<String> = conn
                .query_row(
                    "SELECT id FROM sessions ORDER BY updated_at DESC LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            match next {
                Some(nid) => meta_set(conn, "active_session_id", &nid)?,
                None => meta_delete(conn, "active_session_id")?,
            }
        }
        Ok(())
    })
}

pub fn set_active_session(app: &AppHandle, id: String) -> Result<Session, String> {
    with_db(app, |conn| {
        let session = load_session(conn, &id)?
            .ok_or_else(|| format!("Session not found: {id}"))?;
        meta_set(conn, "active_session_id", &id)?;
        Ok(session)
    })
}

pub fn get_active_session(app: &AppHandle) -> Result<Option<Session>, String> {
    with_db(app, |conn| {
        let Some(id) = meta_get(conn, "active_session_id")? else {
            return Ok(None);
        };
        load_session(conn, &id)
    })
}

pub fn set_global_rules(app: &AppHandle, content: String) -> Result<(), String> {
    with_db(app, |conn| meta_set(conn, "global_rules", &content))
}
