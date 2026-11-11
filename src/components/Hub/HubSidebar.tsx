import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  useChatStore,
  useSessionStore,
  useSettingsStore,
  useWorkspaceStore,
} from "../../stores";
import { getPresetLabel } from "../../lib/types";
import {
  buildRepoTree,
  folderIdsToReveal,
  type RepoTreeNode,
} from "../../lib/repoTree";
import { getDefaultWorkspacePath } from "../../lib/tauri";

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

  const [reposExpanded, setReposExpanded] = useState(true);
  const [openFolders, setOpenFolders] = useState<Set<string>>(() => new Set());

  const filteredRepos = useMemo(
    () =>
      repositories.filter((r) => {
        const q = repoFilter.trim().toLowerCase();
        if (!q) return true;
        return (
          r.name.toLowerCase().includes(q) || r.path.toLowerCase().includes(q)
        );
      }),
    [repositories, repoFilter],
  );

  const repoTree = useMemo(
    () => buildRepoTree(filteredRepos),
    [filteredRepos],
  );

  useEffect(() => {
    const reveal = folderIdsToReveal(repoTree, activeRepositoryId);
    if (reveal.length === 0) return;
    setOpenFolders((prev) => {
      const next = new Set(prev);
      let changed = false;
      for (const id of reveal) {
        if (!next.has(id)) {
          next.add(id);
          changed = true;
        }
      }
      return changed ? next : prev;
    });
  }, [repoTree, activeRepositoryId]);

  async function handleNewAgent() {
    if (streaming) return;
    await createNew(activeRepositoryId);
  }

  async function handleAddRepo() {
    let defaultPath: string | undefined;
    try {
      defaultPath = await getDefaultWorkspacePath();
    } catch {
      defaultPath = path ?? undefined;
    }
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Add repository",
      defaultPath,
    });
    if (typeof selected === "string") {
      await addRepo(selected);
    }
  }

  async function handleSelectRepo(id: string) {
    if (streaming) return;
    await selectRepo(id);
  }

  function toggleFolder(id: string) {
    setOpenFolders((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function renderRepoNode(node: RepoTreeNode, depth: number) {
    const pad = { paddingLeft: `${8 + depth * 12}px` };

    if (node.kind === "folder") {
      const open = openFolders.has(node.id);
      return (
        <div key={node.id}>
          <button
            type="button"
            onClick={() => toggleFolder(node.id)}
            style={pad}
            className="flex w-full items-center gap-1.5 rounded-lg px-2 py-1.5 text-left text-sm text-[var(--text-muted)] hover:bg-[var(--bg)] hover:text-[var(--text)]"
          >
            <span
              aria-hidden
              className={`inline-block w-3 shrink-0 text-[10px] transition-transform ${
                open ? "rotate-90" : ""
              }`}
            >
              ▸
            </span>
            <span className="min-w-0 truncate font-medium">{node.name}</span>
          </button>
          {open && (
            <div className="space-y-0.5">
              {node.children.map((child) => renderRepoNode(child, depth + 1))}
            </div>
          )}
        </div>
      );
    }

    const active = node.repoId === activeRepositoryId;
    return (
      <div
        key={node.id}
        className={`group flex items-center gap-1 rounded-lg ${
          active ? "bg-[var(--bg-elevated)]" : "hover:bg-[var(--bg)]"
        }`}
      >
        <button
          type="button"
          disabled={streaming}
          onClick={() => handleSelectRepo(node.repoId)}
          style={pad}
          className="flex min-w-0 flex-1 items-center gap-1.5 py-1.5 pr-1 text-left disabled:opacity-50"
        >
          <span aria-hidden className="w-3 shrink-0 text-[10px] text-[var(--text-muted)]">
            ·
          </span>
          <span className="min-w-0 flex-1">
            <span className="block truncate text-sm">{node.name}</span>
            {depth === 0 && (
              <span className="block truncate font-mono text-[10px] text-[var(--text-muted)]">
                {node.path}
              </span>
            )}
          </span>
        </button>
        <button
          type="button"
          onClick={() => removeRepo(node.repoId)}
          className="mr-1 hidden rounded px-1.5 text-xs text-[var(--text-muted)] hover:text-[var(--danger)] group-hover:inline"
          title="Remove repository"
        >
          ×
        </button>
      </div>
    );
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
        <div className="mb-2 px-2 text-xs tracking-wide text-[var(--text-muted)]">
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

        <div className="mb-2 flex items-center justify-between gap-1 px-1">
          <button
            type="button"
            onClick={() => setReposExpanded((v) => !v)}
            className="flex min-w-0 flex-1 items-center gap-1.5 rounded-lg px-1 py-1 text-left text-xs tracking-wide text-[var(--text-muted)] hover:text-[var(--text)]"
          >
            <span
              aria-hidden
              className={`inline-block w-3 text-[10px] transition-transform ${
                reposExpanded ? "rotate-90" : ""
              }`}
            >
              ▸
            </span>
            <span>Repositories</span>
          </button>
          <button
            type="button"
            onClick={handleAddRepo}
            className="rounded px-1.5 text-sm text-[var(--text-muted)] hover:text-[var(--accent)]"
            title="Add repository"
          >
            +
          </button>
        </div>

        {reposExpanded && (
          <>
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
              {repoTree.map((node) => renderRepoNode(node, 0))}
            </div>
          </>
        )}
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
