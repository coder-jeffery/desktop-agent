mod agent;
mod commands;
mod edits;
mod llm;
mod persistence;
mod speech_native;
mod tools;
mod workspace_paths;

use agent::AgentState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let agent_state = AgentState::new();

    commands::register(
        tauri::Builder::default()
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_store::Builder::default().build())
            .manage(agent_state)
            .setup(|app| {
                let db = persistence::init_db(app.handle())?;
                app.manage(db);
                Ok(())
            }),
    )
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
