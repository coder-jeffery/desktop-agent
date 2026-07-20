import { useEffect, useState } from "react";
import { useWorkspaceStore } from "../../stores";
import {
  getRules,
  setGlobalRules,
  setWorkspaceRules,
} from "../../lib/tauri";

interface RulesModalProps {
  open: boolean;
  onClose: () => void;
}

export function RulesModal({ open, onClose }: RulesModalProps) {
  const path = useWorkspaceStore((s) => s.path);
  const [globalRules, setGlobal] = useState("");
  const [workspaceRules, setWorkspace] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<"global" | "workspace">("global");

  useEffect(() => {
    if (!open) return;
    setError(null);
    getRules(path)
      .then((rules) => {
        setGlobal(rules.globalRules ?? "");
        setWorkspace(rules.workspaceRules ?? "");
      })
      .catch((e) => setError(String(e)));
  }, [open, path]);

  if (!open) return null;

  async function handleSave() {
    setSaving(true);
    setError(null);
    try {
      await setGlobalRules(globalRules);
      if (path) {
        await setWorkspaceRules(path, workspaceRules);
      }
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <div className="flex max-h-[90vh] w-full max-w-2xl flex-col rounded-xl border border-[var(--border)] bg-[var(--bg-panel)] shadow-2xl">
        <div className="border-b border-[var(--border)] px-6 py-4">
          <h2 className="text-lg font-semibold">Customize / Rules</h2>
          <p className="mt-1 text-xs text-[var(--text-muted)]">
            Rules are injected into the agent system prompt on every turn.
          </p>
        </div>

        <div className="flex gap-2 border-b border-[var(--border)] px-6 pt-3">
          <button
            type="button"
            onClick={() => setTab("global")}
            className={`border-b-2 px-3 pb-2 text-sm ${
              tab === "global"
                ? "border-[var(--accent)] text-[var(--text)]"
                : "border-transparent text-[var(--text-muted)]"
            }`}
          >
            Global Rules
          </button>
          <button
            type="button"
            onClick={() => setTab("workspace")}
            className={`border-b-2 px-3 pb-2 text-sm ${
              tab === "workspace"
                ? "border-[var(--accent)] text-[var(--text)]"
                : "border-transparent text-[var(--text-muted)]"
            }`}
          >
            Workspace Rules
          </button>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto px-6 py-4">
          {tab === "global" ? (
            <label className="block space-y-2">
              <span className="text-sm text-[var(--text-muted)]">
                Applies to all repositories
              </span>
              <textarea
                value={globalRules}
                onChange={(e) => setGlobal(e.target.value)}
                rows={14}
                placeholder={"例：\n- 回答使用中文\n- 优先写小而可验证的改动\n- 不要自动 push"}
                className="w-full resize-y rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 font-mono text-sm leading-relaxed outline-none focus:border-[var(--accent-dim)]"
              />
            </label>
          ) : (
            <label className="block space-y-2">
              <span className="text-sm text-[var(--text-muted)]">
                {path
                  ? `Saved to ${path}/.desktop-agent/rules.md`
                  : "Select a repository first"}
              </span>
              <textarea
                value={workspaceRules}
                onChange={(e) => setWorkspace(e.target.value)}
                rows={14}
                disabled={!path}
                placeholder={"例：\n- 本项目使用 pnpm\n- 测试命令：pnpm test\n- UI 改动需保持现有设计语言"}
                className="w-full resize-y rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 font-mono text-sm leading-relaxed outline-none focus:border-[var(--accent-dim)] disabled:opacity-50"
              />
            </label>
          )}
        </div>

        {error && (
          <p className="px-6 text-sm text-[var(--danger)]">{error}</p>
        )}

        <div className="flex justify-end gap-2 border-t border-[var(--border)] px-6 py-4">
          <button
            type="button"
            onClick={onClose}
            className="rounded-lg px-4 py-2 text-sm text-[var(--text-muted)] hover:text-[var(--text)]"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleSave}
            disabled={saving}
            className="rounded-lg bg-[var(--accent-dim)] px-4 py-2 text-sm font-medium text-white hover:bg-[var(--accent)] disabled:opacity-50"
          >
            {saving ? "Saving…" : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}
