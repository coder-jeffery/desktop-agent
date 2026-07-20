mod agent;
mod commands;
mod edits;
mod llm;
mod persistence;
mod tools;

use agent::AgentState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let agent_state = AgentState::new();

    commands::register(
        tauri::Builder::default()
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_store::Builder::default().build())
            .manage(agent_state),
    )
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
