import { create } from "zustand";
import type {
  AppSettings,
  ChatMessage,
  ProposedEdit,
  Repository,
  Session,
} from "../lib/types";
import { applyPreset, DEFAULT_PRESET_ID } from "../lib/types";
import type { AgentIntensity } from "../lib/types";
import {
  addRepository as apiAddRepository,
  createSession as apiCreateSession,
  deleteSession as apiDeleteSession,
  listRepositories,
  listSessions,
  removeRepository as apiRemoveRepository,
  setActiveRepository as apiSetActiveRepository,
  setActiveSession as apiSetActiveSession,
  setWorkspace,
  updateSession as apiUpdateSession,
  titleFromMessage,
} from "../lib/tauri";

interface SettingsState {
  settings: AppSettings;
  loaded: boolean;
  requireApproval: boolean;
  intensity: AgentIntensity;
  setSettings: (settings: AppSettings) => void;
  applyPresetById: (presetId: AppSettings["preset_id"]) => void;
  setLoaded: (loaded: boolean) => void;
  setRequireApproval: (value: boolean) => void;
  setIntensity: (value: AgentIntensity) => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  settings: applyPreset(DEFAULT_PRESET_ID),
  loaded: false,
  requireApproval: true,
  intensity: "light",
  setSettings: (settings) => set({ settings }),
  applyPresetById: (presetId) => set({ settings: applyPreset(presetId) }),
  setLoaded: (loaded) => set({ loaded }),
  setRequireApproval: (requireApproval) => set({ requireApproval }),
  setIntensity: (intensity) => set({ intensity }),
}));

interface WorkspaceState {
  repositories: Repository[];
  activeRepositoryId: string | null;
  path: string | null;
  repoFilter: string;
  setRepoFilter: (filter: string) => void;
  hydrate: (
    repositories: Repository[],
    active: Repository | null,
  ) => void;
  addRepo: (path: string) => Promise<Repository>;
  selectRepo: (id: string) => Promise<Repository>;
  removeRepo: (id: string) => Promise<void>;
}

export const useWorkspaceStore = create<WorkspaceState>((set, get) => ({
  repositories: [],
  activeRepositoryId: null,
  path: null,
  repoFilter: "",
  setRepoFilter: (repoFilter) => set({ repoFilter }),
  hydrate: (repositories, active) =>
    set({
      repositories,
      activeRepositoryId: active?.id ?? null,
      path: active?.path ?? null,
    }),
  addRepo: async (path) => {
    const repo = await apiAddRepository(path);
    const repositories = await listRepositories();
    set({
      repositories,
      activeRepositoryId: repo.id,
      path: repo.path,
    });
    return repo;
  },
  selectRepo: async (id) => {
    const repo = await apiSetActiveRepository(id);
    set({
      activeRepositoryId: repo.id,
      path: repo.path,
      repositories: get().repositories.map((r) =>
        r.id === repo.id ? repo : r,
      ),
    });
    return repo;
  },
  removeRepo: async (id) => {
    await apiRemoveRepository(id);
    const repositories = await listRepositories();
    const stillActive = repositories.find(
      (r) => r.id === get().activeRepositoryId,
    );
    if (stillActive) {
      set({ repositories, path: stillActive.path });
    } else if (repositories[0]) {
      const repo = await apiSetActiveRepository(repositories[0].id);
      set({
        repositories,
        activeRepositoryId: repo.id,
        path: repo.path,
      });
    } else {
      set({
        repositories: [],
        activeRepositoryId: null,
        path: null,
      });
    }
  },
}));

interface ChatState {
  messages: ChatMessage[];
  streaming: boolean;
  pendingEdits: ProposedEdit[];
  activeEdit: ProposedEdit | null;
  setMessages: (messages: ChatMessage[]) => void;
  addUserMessage: (content: string) => void;
  appendAssistantDelta: (delta: string) => void;
  addToolCall: (id: string, name: string, args: string) => void;
  addToolResult: (id: string, name: string, output: string) => void;
  finalizeAssistantTurn: () => void;
  setStreaming: (streaming: boolean) => void;
  setPendingEdits: (edits: ProposedEdit[]) => void;
  addPendingEdit: (edit: ProposedEdit) => void;
  removePendingEdit: (id: string) => void;
  setActiveEdit: (edit: ProposedEdit | null) => void;
  clearMessages: () => void;
}

export const useChatStore = create<ChatState>((set) => ({
  messages: [],
  streaming: false,
  pendingEdits: [],
  activeEdit: null,
  setMessages: (messages) =>
    set({ messages, pendingEdits: [], activeEdit: null }),
  addUserMessage: (content) =>
    set((s) => ({
      messages: [...s.messages, { role: "user", content }],
    })),
  appendAssistantDelta: (delta) =>
    set((s) => {
      const messages = [...s.messages];
      const last = messages[messages.length - 1];
      if (last?.role === "assistant" && !last.toolCalls?.length) {
        messages[messages.length - 1] = {
          ...last,
          content: last.content + delta,
        };
      } else {
        messages.push({ role: "assistant", content: delta });
      }
      return { messages };
    }),
  addToolCall: (id, name, argumentsJson) =>
    set((s) => ({
      messages: [
        ...s.messages,
        {
          role: "assistant",
          content: "",
          toolCalls: [{ id, name, arguments: argumentsJson }],
        },
      ],
    })),
  addToolResult: (id, name, output) =>
    set((s) => ({
      messages: [
        ...s.messages,
        {
          role: "tool",
          content: output,
          toolCallId: id,
          name,
        },
      ],
    })),
  finalizeAssistantTurn: () => set({ streaming: false }),
  setStreaming: (streaming) => set({ streaming }),
  setPendingEdits: (pendingEdits) => set({ pendingEdits }),
  addPendingEdit: (edit) =>
    set((s) => ({
      pendingEdits: [...s.pendingEdits.filter((e) => e.id !== edit.id), edit],
      activeEdit: edit,
    })),
  removePendingEdit: (id) =>
    set((s) => ({
      pendingEdits: s.pendingEdits.filter((e) => e.id !== id),
      activeEdit: s.activeEdit?.id === id ? null : s.activeEdit,
    })),
  setActiveEdit: (activeEdit) => set({ activeEdit }),
  clearMessages: () =>
    set({ messages: [], pendingEdits: [], activeEdit: null }),
}));

interface SessionState {
  sessions: Session[];
  activeSessionId: string | null;
  hydrate: (sessions: Session[], active: Session | null) => void;
  createNew: (workspaceId?: string | null) => Promise<Session>;
  select: (id: string) => Promise<Session>;
  remove: (id: string) => Promise<void>;
  persistActive: (messages: ChatMessage[], titleHint?: string) => Promise<void>;
}

export const useSessionStore = create<SessionState>((set, get) => ({
  sessions: [],
  activeSessionId: null,
  hydrate: (sessions, active) =>
    set({
      sessions,
      activeSessionId: active?.id ?? sessions[0]?.id ?? null,
    }),
  createNew: async (workspaceId) => {
    const current = get().sessions.find((s) => s.id === get().activeSessionId);
    if (current) {
      await apiUpdateSession({
        ...current,
        messages: useChatStore.getState().messages,
      });
    }
    const session = await apiCreateSession(workspaceId ?? null, "New Agent");
    const sessions = await listSessions();
    useChatStore.getState().clearMessages();
    set({ sessions, activeSessionId: session.id });
    return session;
  },
  select: async (id) => {
    const currentId = get().activeSessionId;
    if (currentId && currentId !== id) {
      const current = get().sessions.find((s) => s.id === currentId);
      if (current) {
        await apiUpdateSession({
          ...current,
          messages: useChatStore.getState().messages,
        });
      }
    }
    const session = await apiSetActiveSession(id);
    useChatStore.getState().setMessages(session.messages ?? []);

    if (session.workspaceId) {
      const repos = useWorkspaceStore.getState().repositories;
      const repo = repos.find((r) => r.id === session.workspaceId);
      if (repo) {
        await useWorkspaceStore.getState().selectRepo(repo.id);
      }
    }

    const sessions = await listSessions();
    set({ sessions, activeSessionId: session.id });
    return session;
  },
  remove: async (id) => {
    await apiDeleteSession(id);
    const sessions = await listSessions();
    const next = sessions[0] ?? null;
    if (next) {
      await apiSetActiveSession(next.id);
      useChatStore.getState().setMessages(next.messages ?? []);
      set({ sessions, activeSessionId: next.id });
    } else {
      const created = await apiCreateSession(
        useWorkspaceStore.getState().activeRepositoryId,
        "New Agent",
      );
      useChatStore.getState().clearMessages();
      set({
        sessions: await listSessions(),
        activeSessionId: created.id,
      });
    }
  },
  persistActive: async (messages, titleHint) => {
    const id = get().activeSessionId;
    if (!id) return;
    const current = get().sessions.find((s) => s.id === id);
    if (!current) return;

    const shouldRename =
      current.title === "New Agent" && titleHint && titleHint.trim();
    const updated = await apiUpdateSession({
      ...current,
      title: shouldRename ? titleFromMessage(titleHint!) : current.title,
      workspaceId:
        current.workspaceId ??
        useWorkspaceStore.getState().activeRepositoryId,
      messages,
    });
    set((s) => ({
      sessions: s.sessions
        .map((sess) => (sess.id === updated.id ? updated : sess))
        .sort((a, b) => b.updatedAt - a.updatedAt),
    }));
  },
}));

export async function bootstrapAppData() {
  const { getActiveRepository, getActiveSession } = await import("../lib/tauri");
  const repositories = await listRepositories();
  let activeRepo = (await getActiveRepository()) ?? repositories[0] ?? null;

  if (activeRepo) {
    await setWorkspace(activeRepo.path);
  }
  useWorkspaceStore.getState().hydrate(repositories, activeRepo);

  let sessions = await listSessions();
  let active = await getActiveSession();

  if (sessions.length === 0) {
    active = await apiCreateSession(activeRepo?.id ?? null, "New Agent");
    sessions = await listSessions();
  } else if (!active) {
    active = sessions[0];
    await apiSetActiveSession(active.id);
  }

  useSessionStore.getState().hydrate(sessions, active);
  useChatStore.getState().setMessages(active?.messages ?? []);

  if (active?.workspaceId) {
    const linked = repositories.find((r) => r.id === active!.workspaceId);
    if (linked && linked.id !== activeRepo?.id) {
      await useWorkspaceStore.getState().selectRepo(linked.id);
    }
  }
}
