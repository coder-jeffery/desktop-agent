import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  ChatMessage,
  ProposedEdit,
  Repository,
  RulesBundle,
  SendMessageRequest,
  Session,
} from "./types";

export async function getSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("get_settings");
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  return invoke("save_settings", { settings });
}

export async function setWorkspace(path: string): Promise<void> {
  return invoke("set_workspace", { path });
}

export async function getWorkspace(): Promise<string | null> {
  return invoke<string | null>("get_workspace");
}

export async function sendMessage(request: SendMessageRequest): Promise<void> {
  return invoke("send_message", { request });
}

export async function cancelAgent(): Promise<void> {
  return invoke("cancel_agent");
}

export async function listPendingEdits(): Promise<ProposedEdit[]> {
  return invoke<ProposedEdit[]>("list_pending_edits");
}

export async function applyEdit(editId: string): Promise<void> {
  return invoke("apply_edit", { editId });
}

export async function rejectEdit(editId: string): Promise<void> {
  return invoke("reject_edit", { editId });
}

export async function listOllamaModels(
  baseUrl?: string | null,
): Promise<string[]> {
  return invoke<string[]>("list_ollama_models", { baseUrl: baseUrl ?? null });
}

export async function getGitBranch(
  path?: string | null,
): Promise<string> {
  return invoke<string>("get_git_branch", { path: path ?? null });
}

export async function listRepositories(): Promise<Repository[]> {
  return invoke<Repository[]>("list_repositories");
}

export async function addRepository(path: string): Promise<Repository> {
  return invoke<Repository>("add_repository", { path });
}

export async function removeRepository(id: string): Promise<void> {
  return invoke("remove_repository", { id });
}

export async function setActiveRepository(id: string): Promise<Repository> {
  return invoke<Repository>("set_active_repository", { id });
}

export async function getActiveRepository(): Promise<Repository | null> {
  return invoke<Repository | null>("get_active_repository");
}

export async function listSessions(): Promise<Session[]> {
  return invoke<Session[]>("list_sessions");
}

export async function createSession(
  workspaceId?: string | null,
  title?: string | null,
): Promise<Session> {
  return invoke<Session>("create_session", {
    workspaceId: workspaceId ?? null,
    title: title ?? null,
  });
}

export async function updateSession(session: Session): Promise<Session> {
  return invoke<Session>("update_session", { session });
}

export async function deleteSession(id: string): Promise<void> {
  return invoke("delete_session", { id });
}

export async function setActiveSession(id: string): Promise<Session> {
  return invoke<Session>("set_active_session", { id });
}

export async function getActiveSession(): Promise<Session | null> {
  return invoke<Session | null>("get_active_session");
}

export async function getRules(
  workspacePath?: string | null,
): Promise<RulesBundle> {
  return invoke<RulesBundle>("get_rules", {
    workspacePath: workspacePath ?? null,
  });
}

export async function setGlobalRules(content: string): Promise<void> {
  return invoke("set_global_rules", { content });
}

export async function setWorkspaceRules(
  workspacePath: string,
  content: string,
): Promise<void> {
  return invoke("set_workspace_rules", { workspacePath, content });
}

export async function transcribeAudio(
  audioBase64: string,
  locale?: string | null,
): Promise<string> {
  return invoke<string>("transcribe_audio", {
    audioBase64,
    locale: locale ?? "zh-CN",
  });
}

export function toBackendHistory(messages: ChatMessage[]) {
  return messages.map((m) => ({
    role: m.role,
    content: m.content,
    tool_calls: m.toolCalls?.map((tc) => ({
      id: tc.id,
      name: tc.name,
      arguments: tc.arguments,
    })),
    tool_call_id: m.toolCallId,
    name: m.name,
  }));
}

export function titleFromMessage(content: string): string {
  const trimmed = content.trim().replace(/\s+/g, " ");
  if (!trimmed) return "New Agent";
  return trimmed.length > 42 ? `${trimmed.slice(0, 42)}…` : trimmed;
}
