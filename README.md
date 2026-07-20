# Desktop Agent

A local desktop coding assistant built with **Tauri 2**, **React**, and **Rust**.

Chat with an AI agent that can read and search your workspace, propose file edits (with diff preview), run shell commands, and perform basic Git operations — all constrained to the selected project folder.

## Features

- **Multi-model**: Anthropic Claude, OpenAI, domestic providers (DeepSeek, Qwen, Moonshot, ZhiPu, SiliconFlow), and local **Ollama**
- **Streaming responses** with live tool-call visibility
- **Tools**: `read_file`, `write_file`, `init_project`, `list_dir`, `glob`, `grep`, `run_shell`, `git_status`, `git_diff`, `git_commit`
- **Diff preview** before applying file changes
- **Workspace sandbox** — tools cannot access paths outside the selected folder

## Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [pnpm](https://pnpm.io/)
- [Rust](https://rustlang.org/) stable
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS

## Sessions, Repositories & Rules

- **New Agent (⌘N)**: create a new chat session bound to the active repository
- **Agents list**: switch or delete sessions; full chat history is stored in a local **SQLite** database and restored on next launch
- **Repositories**: add multiple project folders, filter, switch active workspace
- **Voice input**: microphone button in the composer (Web Speech API, zh-CN)
- **Customize / Rules**:
  - Global rules (all repos)
  - Workspace rules saved to `.desktop-agent/rules.md` in the selected repo
  - Both are injected into the agent system prompt

## Development

```bash
pnpm install
pnpm tauri dev
```

## Build

```bash
pnpm tauri build
```

Installers are written to `src-tauri/target/release/bundle/`.

## Usage

1. Launch the app (defaults to **Ollama** at `http://localhost:11434/v1`, model `llama3.2`)
2. **Settings** → change provider if needed; Ollama needs no API key
3. **Choose project folder** in the sidebar
4. Ask the agent to explore, edit, test, or commit code

### Provider presets

| Preset | Base URL (default) | API Key |
|--------|-------------------|---------|
| DeepSeek | `https://api.deepseek.com/v1` | Required |
| 通义千问 | `https://dashscope.aliyuncs.com/compatible-mode/v1` | Required |
| Moonshot / Kimi | `https://api.moonshot.cn/v1` | Required |
| 智谱 GLM | `https://open.bigmodel.cn/api/paas/v4` | Required |
| SiliconFlow | `https://api.siliconflow.cn/v1` | Required |
| Ollama (local) | `http://localhost:11434/v1` | Optional |

For Ollama, click **从 Ollama 拉取模型列表** in Settings after `ollama serve` is running.

### Example prompts

- "List all TypeScript files and summarize the project structure"
- "Find usages of `useState` in src/"
- "Add error handling to the fetch helper in `src/api.ts`"
- "Run `cargo test` and explain any failures"
- "Show git status and stage a commit for the current changes"

## Architecture

```
React UI  ←invoke/events→  Rust Agent Core
                              ├── LLM providers (Anthropic / OpenAI-compat)
                              ├── Tool runtime (FS, shell, git, search)
                              ├── Proposed edit queue
                              └── SQLite (sessions, messages, repositories)
```

Conversation history lives in `desktop_agent.db` under the app data directory. API keys are stored locally via Tauri Store (not committed to git). Existing `app_data.json` is migrated into SQLite on first launch.

## Security notes

- Shell commands run in the workspace with a basic blocklist and 30s timeout
- File writes require explicit user approval via the diff panel
- `git push` is not exposed in v1
