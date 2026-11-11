import { useEffect, useState } from "react";
import { ChatPanel } from "./components/Chat/ChatPanel";
import { DiffPreview } from "./components/DiffPreview/DiffPreview";
import { HubSidebar } from "./components/Hub/HubSidebar";
import { RulesModal } from "./components/Rules/RulesModal";
import {
  SettingsModal,
  loadSettingsOnBoot,
} from "./components/Settings/SettingsModal";
import {
  bootstrapAppData,
  useChatStore,
  useSessionStore,
  useSettingsStore,
  useWorkspaceStore,
} from "./stores";
import { isApiKeyRequired } from "./lib/types";

function App() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [rulesOpen, setRulesOpen] = useState(false);
  const [booted, setBooted] = useState(false);
  const [bootError, setBootError] = useState<string | null>(null);
  const { settings, requireApproval } = useSettingsStore();
  const { activeEdit } = useChatStore();
  const createNew = useSessionStore((s) => s.createNew);
  const activeRepositoryId = useWorkspaceStore((s) => s.activeRepositoryId);
  const streaming = useChatStore((s) => s.streaming);

  useEffect(() => {
    (async () => {
      try {
        await loadSettingsOnBoot();
        await bootstrapAppData();
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        console.error("Boot failed:", err);
        setBootError(msg);
      } finally {
        setBooted(true);
      }
    })();
  }, []);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "n") {
        e.preventDefault();
        if (!streaming) {
          createNew(activeRepositoryId).catch(console.error);
        }
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [createNew, activeRepositoryId, streaming]);

  if (!booted) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-[var(--text-muted)]">
        Loading…
      </div>
    );
  }

  if (bootError) {
    const missingRepo = bootError.includes("Not a directory");
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 px-6 text-center text-sm">
        <p className="text-[var(--danger)]">启动失败</p>
        <p className="max-w-md break-all text-[var(--text-muted)]">
          {bootError}
        </p>
        {missingRepo ? (
          <p className="max-w-md text-xs text-[var(--text-muted)]">
            本地数据库里记录的仓库路径已不存在（例如换过电脑或移动过项目）。重启
            Tauri 后会自动跳过无效路径；在侧栏删除旧仓库并重新添加当前项目目录即可。
          </p>
        ) : (
          <p className="max-w-md text-xs text-[var(--text-muted)]">
            请用 <code className="text-[var(--text)]">pnpm tauri dev</code>{" "}
            打开桌面窗口（不要只访问浏览器里的 localhost:1420）。
          </p>
        )}
      </div>
    );
  }

  return (
    <div className="flex h-full">
      <HubSidebar
        onOpenSettings={() => setSettingsOpen(true)}
        onOpenRules={() => setRulesOpen(true)}
      />

      <main className="relative flex min-w-0 flex-1 flex-col bg-[radial-gradient(ellipse_at_bottom,_#151b24_0%,_var(--bg)_55%)]">
        {!isApiKeyRequired(settings) || settings.api_key ? null : (
          <div className="absolute right-4 top-3 z-10">
            <button
              type="button"
              onClick={() => setSettingsOpen(true)}
              className="rounded-full border border-[var(--warning)]/40 bg-[var(--bg-panel)] px-3 py-1 text-xs text-[var(--warning)]"
            >
              需要配置 API Key
            </button>
          </div>
        )}

        <div className="min-h-0 flex-1">
          <ChatPanel onOpenSettings={() => setSettingsOpen(true)} />
        </div>

        {requireApproval && activeEdit && <DiffPreview edit={activeEdit} />}
      </main>

      <SettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
      />
      <RulesModal open={rulesOpen} onClose={() => setRulesOpen(false)} />
    </div>
  );
}

export default App;
