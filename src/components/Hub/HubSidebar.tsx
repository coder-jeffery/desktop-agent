import { open } from "@tauri-apps/plugin-dialog";
import {
  useChatStore,
  useSessionStore,
  useSettingsStore,
  useWorkspaceStore,
} from "../../stores";
import { getPresetLabel } from "../../lib/types";

interface HubSidebarProps {
  onOpenSettings: () => void;
  onOpenRules: () => void;
}

export function HubSidebar({ onOpenSettings, onOpenRules }: HubSidebarProps) {
  const { settings } = useSettingsStore();
  const {
    repositories,
    activeRepositoryId,
    path,
    repoFilter,
    setRepoFilter,
    addRepo,
    selectRepo,
    removeRepo,
  } = useWorkspaceStore();
  const { sessions, activeSessionId, createNew, select, remove } =
    useSessionStore();
  const streaming = useChatStore((s) => s.streaming);

  const filteredRepos = repositories.filter((r) => {
    const q = repoFilter.trim().toLowerCase();
    if (!q) return true;
    return (
      r.name.toLowerCase().includes(q) || r.path.toLowerCase().includes(q)
    );
  });

  async function handleNewAgent() {
    if (streaming) return;
    await createNew(activeRepositoryId);
  }

  async function handleAddRepo() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Add repository",
    });
    if (typeof selected === "string") {
      await addRepo(selected);
    }
  }

  async function handleSelectRepo(id: string) {
    if (streaming) return;
    await selectRepo(id);
  }

  return (
    <aside className="flex w-72 shrink-0 flex-col border-r border-[var(--border)] bg-[var(--bg-panel)]">
      <div className="border-b border-[var(--border)] p-4">
        <div className="text-lg font-semibold tracking-tight">Desktop Agent</div>
        <div className="text-xs text-[var(--text-muted)]">
          Local coding assistant
        </div>
      </div>

      <div className="space-y-1 border-b border-[var(--border)] p-2">
        <button
          type="button"
          onClick={handleNewAgent}
          disabled={streaming}
          className="flex w-full items-center justify-between rounded-lg bg-[var(--bg-elevated)] px-3 py-2 text-sm hover:border-[var(--accent-dim)] disabled:opacity-50"
        >
          <span className="flex items-center gap-2">
            <span aria-hidden>✈</span>
            New Agent
          </span>
          <span className="text-xs text-[var(--text-muted)]">⌘N</span>
        </button>
        <button
          type="button"
          onClick={onOpenRules}
          className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm text-[var(--text-muted)] hover:bg-[var(--bg-elevated)] hover:text-[var(--text)]"
        >
          <span aria-hidden>▦</span>
          Customize / Rules
        </button>
        <button
          type="button"
          onClick={onOpenSettings}
          className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm text-[var(--text-muted)] hover:bg-[var(--bg-elevated)] hover:text-[var(--text)]"
        >
          <span aria-hidden>⚙</span>
          Model Settings
        </button>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        <div className="mb-2 px-2 text-xs uppercase tracking-wider text-[var(--text-muted)]">
          Agents
        </div>
        <div className="mb-4 space-y-0.5">
          {sessions.length === 0 && (
            <div className="px-2 py-3 text-xs text-[var(--text-muted)]">
              No sessions yet
            </div>
          )}
          {sessions.map((session) => {
            const active = session.id === activeSessionId;
            return (
              <div
                key={session.id}
                className={`group flex items-center gap-1 rounded-lg ${
                  active ? "bg-[var(--bg-elevated)]" : "hover:bg-[var(--bg)]"
                }`}
              >
                <button
                  type="button"
                  disabled={streaming}
                  onClick={() => select(session.id)}
                  className="min-w-0 flex-1 truncate px-3 py-2 text-left text-sm disabled:opacity-50"
                  title={session.title}
                >
                  {session.title}
                </button>
                <button
                  type="button"
                  disabled={streaming}
                  onClick={() => remove(session.id)}
                  className="mr-1 hidden rounded px-1.5 text-xs text-[var(--text-muted)] hover:text-[var(--danger)] group-hover:inline"
                  title="Delete session"
                >
                  ×
                </button>
              </div>
            );
          })}
        </div>

        <div className="mb-2 flex items-center justify-between px-2">
          <div className="text-xs uppercase tracking-wider text-[var(--text-muted)]">
            Repositories
          </div>
          <button
            type="button"
            onClick={handleAddRepo}
            className="rounded px-1.5 text-sm text-[var(--text-muted)] hover:text-[var(--accent)]"
            title="Add repository"
          >
            +
          </button>
        </div>

        {repositories.length > 3 && (
          <input
            value={repoFilter}
            onChange={(e) => setRepoFilter(e.target.value)}
            placeholder="Filter…"
            className="mb-2 w-full rounded-lg border border-[var(--border)] bg-[var(--bg)] px-2 py-1.5 text-xs outline-none focus:border-[var(--accent-dim)]"
          />
        )}

        <div className="space-y-0.5">
          {filteredRepos.length === 0 && (
            <button
              type="button"
              onClick={handleAddRepo}
              className="w-full rounded-lg border border-dashed border-[var(--border)] px-3 py-4 text-left text-xs text-[var(--text-muted)] hover:border-[var(--accent-dim)]"
            >
              Add a project folder…
            </button>
          )}
          {filteredRepos.map((repo) => {
            const active = repo.id === activeRepositoryId;
            return (
              <div
                key={repo.id}
                className={`group flex items-center gap-1 rounded-lg ${
                  active ? "bg-[var(--bg-elevated)]" : "hover:bg-[var(--bg)]"
                }`}
              >
                <button
                  type="button"
                  disabled={streaming}
                  onClick={() => handleSelectRepo(repo.id)}
                  className="min-w-0 flex-1 px-3 py-2 text-left disabled:opacity-50"
                >
                  <div className="truncate text-sm">{repo.name}</div>
                  <div className="truncate font-mono text-[10px] text-[var(--text-muted)]">
                    {repo.path}
                  </div>
                </button>
                <button
                  type="button"
                  onClick={() => removeRepo(repo.id)}
                  className="mr-1 hidden rounded px-1.5 text-xs text-[var(--text-muted)] hover:text-[var(--danger)] group-hover:inline"
                  title="Remove repository"
                >
                  ×
                </button>
              </div>
            );
          })}
        </div>
      </div>

      <div className="border-t border-[var(--border)] p-3">
        <div className="mb-1 text-xs text-[var(--text-muted)]">
          {getPresetLabel(settings)}
        </div>
        <div className="truncate font-mono text-xs text-[var(--text-muted)]">
          {settings.model}
        </div>
        <div className="mt-2 truncate font-mono text-[10px] text-[var(--text-muted)]">
          {path ?? "No repository selected"}
        </div>
      </div>
    </aside>
  );
}
