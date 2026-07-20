import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  useSettingsStore,
  useWorkspaceStore,
} from "../../stores";
import { getGitBranch } from "../../lib/tauri";
import { useSpeechInput } from "../../lib/useSpeechInput";
import {
  INTENSITY_LABELS,
  MODEL_PRESETS,
  PRESET_GROUPS,
  applyPreset,
  getPresetLabel,
  intensityHint,
  shortModelLabel,
  type AgentIntensity,
  type ModelPresetId,
} from "../../lib/types";
import { saveSettings } from "../../lib/tauri";

interface ComposerBarProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  onStop: () => void;
  streaming: boolean;
  canSend: boolean;
  onOpenSettings: () => void;
  attachments: string[];
  onAttachmentsChange: (paths: string[]) => void;
}

export function ComposerBar({
  value,
  onChange,
  onSubmit,
  onStop,
  streaming,
  canSend,
  onOpenSettings,
  attachments,
  onAttachmentsChange,
}: ComposerBarProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const modelMenuRef = useRef<HTMLDivElement>(null);
  const valueRef = useRef(value);
  valueRef.current = value;

  const [branch, setBranch] = useState("—");
  const [modelOpen, setModelOpen] = useState(false);

  const {
    settings,
    requireApproval,
    intensity,
    setRequireApproval,
    setIntensity,
    setSettings,
  } = useSettingsStore();
  const { path, repositories, activeRepositoryId } = useWorkspaceStore();

  const speech = useSpeechInput({
    lang: "zh-CN",
    disabled: streaming || !path,
    onFinal: (transcript) => {
      const base = valueRef.current;
      const sep = base && !base.endsWith(" ") ? " " : "";
      onChange(base ? `${base}${sep}${transcript}` : transcript);
      textareaRef.current?.focus();
    },
  });

  const repoName =
    repositories.find((r) => r.id === activeRepositoryId)?.name ??
    (path ? path.split(/[/\\]/).filter(Boolean).pop() : null) ??
    "未选择";

  useEffect(() => {
    let cancelled = false;
    if (!path) {
      setBranch("—");
      return;
    }
    getGitBranch(path)
      .then((b) => {
        if (!cancelled) setBranch(b || "—");
      })
      .catch(() => {
        if (!cancelled) setBranch("—");
      });
    return () => {
      cancelled = true;
    };
  }, [path]);

  useEffect(() => {
    function onDocClick(e: MouseEvent) {
      if (!modelMenuRef.current?.contains(e.target as Node)) {
        setModelOpen(false);
      }
    }
    if (modelOpen) {
      document.addEventListener("mousedown", onDocClick);
      return () => document.removeEventListener("mousedown", onDocClick);
    }
  }, [modelOpen]);

  async function handleAddContext() {
    const selected = await open({
      multiple: true,
      title: "添加上下文文件",
      defaultPath: path ?? undefined,
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    const next = [...attachments];
    for (const p of paths) {
      if (!next.includes(p)) next.push(p);
    }
    onAttachmentsChange(next);
  }

  function removeAttachment(p: string) {
    onAttachmentsChange(attachments.filter((a) => a !== p));
  }

  async function selectPreset(id: ModelPresetId) {
    const next = {
      ...applyPreset(id),
      api_key: settings.api_key,
    };
    if (id === "ollama") {
      next.api_key = settings.provider === "ollama" ? settings.api_key : "";
    } else if (settings.provider !== "ollama") {
      next.api_key = settings.api_key;
    }
    setSettings(next);
    try {
      await saveSettings(next);
    } catch (e) {
      console.error(e);
    }
    setModelOpen(false);
  }

  const modelLabel = `${shortModelLabel(settings.model)} ${INTENSITY_LABELS[intensity]}`;

  const micTitle = !speech.isSupported
    ? "当前环境不支持语音输入"
    : speech.isListening
      ? "点击停止语音输入"
      : speech.isRequesting
        ? "正在请求麦克风权限…"
        : "语音输入";

  return (
    <div className="mx-auto w-full max-w-3xl px-4 pb-5 pt-2">
      {/* Context strip */}
      <div className="mb-2 flex flex-wrap items-center gap-3 rounded-2xl border border-[var(--border)] bg-[#1a1f27]/80 px-3.5 py-2 text-xs text-[var(--text-muted)]">
        <span className="inline-flex max-w-[40%] items-center gap-1.5 truncate">
          <FolderIcon />
          <span className="truncate text-[var(--text)]">{repoName}</span>
        </span>
        <span className="inline-flex items-center gap-1.5">
          <DesktopIcon />
          <span>本地</span>
        </span>
        <span className="inline-flex items-center gap-1.5">
          <BranchIcon />
          <span className="font-mono text-[var(--text)]">{branch}</span>
        </span>
      </div>

      {/* Composer card */}
      <div className="overflow-hidden rounded-2xl border border-[var(--border)] bg-[#1c222b] shadow-[0_8px_30px_rgba(0,0,0,0.35)]">
        {attachments.length > 0 && (
          <div className="flex flex-wrap gap-1.5 border-b border-[var(--border)] px-3 py-2">
            {attachments.map((p) => (
              <span
                key={p}
                className="inline-flex max-w-full items-center gap-1 rounded-full bg-[var(--bg)] px-2 py-0.5 font-mono text-[10px] text-[var(--text-muted)]"
              >
                <span className="truncate">{p.split(/[/\\]/).pop()}</span>
                <button
                  type="button"
                  onClick={() => removeAttachment(p)}
                  className="text-[var(--text-muted)] hover:text-[var(--danger)]"
                >
                  ×
                </button>
              </span>
            ))}
          </div>
        )}

        {speech.error && (
          <div className="flex items-center justify-between gap-2 border-b border-[var(--border)] bg-[var(--danger)]/10 px-3 py-2 text-xs text-[var(--danger)]">
            <span>{speech.error}</span>
            <button
              type="button"
              onClick={speech.clearError}
              className="shrink-0 rounded px-1.5 py-0.5 hover:bg-[var(--danger)]/20"
            >
              关闭
            </button>
          </div>
        )}

        <div className="relative">
          <textarea
            ref={textareaRef}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                if (canSend) onSubmit();
              }
            }}
            rows={3}
            placeholder={path ? "随心输入，或点击麦克风语音输入" : "请先在侧栏添加仓库…"}
            disabled={streaming || !path}
            className={`w-full resize-none bg-transparent px-4 pt-3.5 text-[15px] leading-relaxed text-[var(--text)] outline-none placeholder:text-[#6b7280] disabled:opacity-50 ${
              speech.isListening ? "pb-8" : "pb-2"
            }`}
          />
          {speech.isListening && (
            <div className="pointer-events-none absolute bottom-2 left-4 right-4 flex items-center gap-1.5 truncate text-xs text-[var(--accent)]">
              <span className="relative flex h-2 w-2 shrink-0">
                <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[var(--accent)] opacity-60" />
                <span className="relative inline-flex h-2 w-2 rounded-full bg-[var(--accent)]" />
              </span>
              <span className="truncate">
                {speech.interimText || "正在聆听…"}
              </span>
            </div>
          )}
        </div>

        <div className="flex items-center justify-between gap-2 px-3 pb-3">
          <div className="flex min-w-0 items-center gap-1">
            <button
              type="button"
              onClick={handleAddContext}
              disabled={!path || streaming}
              title="添加上下文文件"
              className="flex h-8 w-8 items-center justify-center rounded-lg text-[var(--text-muted)] hover:bg-[var(--bg)] hover:text-[var(--text)] disabled:opacity-40"
            >
              <PlusIcon />
            </button>

            <button
              type="button"
              onClick={() => setRequireApproval(!requireApproval)}
              title={
                requireApproval
                  ? "写入前需要人工批准（当前开启）"
                  : "自动应用文件改动（当前关闭批准）"
              }
              className={`inline-flex h-8 items-center gap-1.5 rounded-lg px-2.5 text-xs transition ${
                requireApproval
                  ? "bg-[#2a3340] text-[var(--text)]"
                  : "text-[var(--text-muted)] hover:bg-[var(--bg)]"
              }`}
            >
              <HandIcon />
              请求批准
            </button>
          </div>

          <div className="flex items-center gap-1">
            <div className="relative" ref={modelMenuRef}>
              <button
                type="button"
                onClick={() => setModelOpen((o) => !o)}
                className="inline-flex h-8 max-w-[200px] items-center gap-1 rounded-lg px-2 text-xs text-[var(--text-muted)] hover:bg-[var(--bg)] hover:text-[var(--text)]"
                title={getPresetLabel(settings)}
              >
                <span className="truncate">{modelLabel}</span>
                <ChevronIcon />
              </button>

              {modelOpen && (
                <div className="absolute bottom-full right-0 z-20 mb-2 w-64 overflow-hidden rounded-xl border border-[var(--border)] bg-[var(--bg-panel)] shadow-xl">
                  <div className="border-b border-[var(--border)] px-3 py-2 text-[10px] uppercase tracking-wider text-[var(--text-muted)]">
                    强度
                  </div>
                  <div className="flex gap-1 p-2">
                    {(Object.keys(INTENSITY_LABELS) as AgentIntensity[]).map(
                      (key) => (
                        <button
                          key={key}
                          type="button"
                          title={intensityHint(key)}
                          onClick={() => setIntensity(key)}
                          className={`flex-1 rounded-lg px-2 py-1.5 text-xs ${
                            intensity === key
                              ? "bg-[var(--accent-dim)] text-white"
                              : "bg-[var(--bg)] text-[var(--text-muted)] hover:text-[var(--text)]"
                          }`}
                        >
                          {INTENSITY_LABELS[key]}
                        </button>
                      ),
                    )}
                  </div>

                  <div className="max-h-56 overflow-y-auto border-t border-[var(--border)] py-1">
                    {PRESET_GROUPS.map((group) => (
                      <div key={group.key}>
                        <div className="px-3 py-1.5 text-[10px] uppercase tracking-wider text-[var(--text-muted)]">
                          {group.label}
                        </div>
                        {MODEL_PRESETS.filter((p) => p.group === group.key).map(
                          (preset) => (
                            <button
                              key={preset.id}
                              type="button"
                              onClick={() => selectPreset(preset.id)}
                              className={`flex w-full items-center justify-between px-3 py-1.5 text-left text-xs hover:bg-[var(--bg-elevated)] ${
                                settings.preset_id === preset.id
                                  ? "text-[var(--accent)]"
                                  : "text-[var(--text)]"
                              }`}
                            >
                              <span>{preset.label}</span>
                              {settings.preset_id === preset.id && (
                                <span>·</span>
                              )}
                            </button>
                          ),
                        )}
                      </div>
                    ))}
                  </div>

                  <button
                    type="button"
                    onClick={() => {
                      setModelOpen(false);
                      onOpenSettings();
                    }}
                    className="w-full border-t border-[var(--border)] px-3 py-2 text-left text-xs text-[var(--text-muted)] hover:bg-[var(--bg-elevated)] hover:text-[var(--text)]"
                  >
                    更多模型设置…
                  </button>
                </div>
              )}
            </div>

            <button
              type="button"
              onClick={() => speech.toggle()}
              disabled={streaming || !path || !speech.isSupported}
              title={micTitle}
              className={`flex h-8 w-8 items-center justify-center rounded-lg hover:bg-[var(--bg)] disabled:opacity-40 ${
                speech.isListening || speech.isRequesting
                  ? "bg-[var(--accent)]/15 text-[var(--accent)] ring-1 ring-[var(--accent)]/40"
                  : "text-[var(--text-muted)] hover:text-[var(--text)]"
              }`}
            >
              <MicIcon active={speech.isListening} />
            </button>

            {streaming ? (
              <button
                type="button"
                onClick={onStop}
                className="flex h-8 w-8 items-center justify-center rounded-full border border-[var(--danger)] text-[var(--danger)] hover:bg-[var(--danger)]/10"
                title="停止"
              >
                <StopIcon />
              </button>
            ) : (
              <button
                type="button"
                onClick={onSubmit}
                disabled={!canSend}
                className="flex h-8 w-8 items-center justify-center rounded-full bg-[#3d4450] text-[var(--text)] transition hover:bg-[var(--accent-dim)] disabled:cursor-not-allowed disabled:opacity-35"
                title="发送"
              >
                <ArrowUpIcon />
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function FolderIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
      <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />
    </svg>
  );
}

function DesktopIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
      <rect x="3" y="4" width="18" height="12" rx="2" />
      <path d="M8 20h8M12 16v4" />
    </svg>
  );
}

function BranchIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
      <circle cx="6" cy="6" r="2" />
      <circle cx="18" cy="6" r="2" />
      <circle cx="6" cy="18" r="2" />
      <path d="M8 6h8M6 8v8M8 18h4a4 4 0 0 0 4-4V8" />
    </svg>
  );
}

function PlusIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M12 5v14M5 12h14" />
    </svg>
  );
}

function HandIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
      <path d="M8 13V5a1 1 0 0 1 2 0v6M10 11V4a1 1 0 0 1 2 0v7M12 10V5a1 1 0 0 1 2 0v7M14 11V7a1 1 0 0 1 2 0v8c0 3-2 5-5 5h-1a5 5 0 0 1-5-5v-3a1 1 0 0 1 2 0v2" />
    </svg>
  );
}

function ChevronIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M6 9l6 6 6-6" />
    </svg>
  );
}

function MicIcon({ active }: { active?: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
      <rect x="9" y="3" width="6" height="11" rx="3" fill={active ? "currentColor" : "none"} />
      <path d="M5 11a7 7 0 0 0 14 0M12 18v3" />
    </svg>
  );
}

function ArrowUpIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2">
      <path d="M12 19V5M5 12l7-7 7 7" />
    </svg>
  );
}

function StopIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
      <rect x="6" y="6" width="12" height="12" rx="1.5" />
    </svg>
  );
}
