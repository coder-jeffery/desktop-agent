import { useEffect, useState } from "react";
import { useSettingsStore } from "../../stores";
import { getSettings, listOllamaModels, saveSettings } from "../../lib/tauri";
import type { AppSettings, ModelPresetId } from "../../lib/types";
import {
  MODEL_PRESETS,
  PRESET_GROUPS,
  applyPreset,
  inferPresetFromSettings,
  isApiKeyRequired,
} from "../../lib/types";

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

export function SettingsModal({ open, onClose }: SettingsModalProps) {
  const { settings, setSettings, setLoaded } = useSettingsStore();
  const [draft, setDraft] = useState<AppSettings>(settings);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ollamaModels, setOllamaModels] = useState<string[]>([]);
  const [loadingOllama, setLoadingOllama] = useState(false);

  const activePreset = inferPresetFromSettings(draft);
  const apiKeyRequired = isApiKeyRequired(draft);
  const showBaseUrl =
    activePreset.baseUrlEditable ||
    draft.provider === "open_ai_compatible" ||
    draft.provider === "ollama";

  useEffect(() => {
    if (open) {
      const normalized = {
        ...settings,
        preset_id: settings.preset_id ?? inferPresetFromSettings(settings).id,
      };
      setDraft(normalized);
      setError(null);
      setOllamaModels([]);
    }
  }, [open, settings]);

  if (!open) return null;

  function selectPreset(presetId: ModelPresetId) {
    const next = applyPreset(presetId);
    setDraft((prev) => ({
      ...next,
      api_key: presetId === prev.preset_id ? prev.api_key : prev.api_key,
    }));
  }

  async function refreshOllamaModels() {
    setLoadingOllama(true);
    setError(null);
    try {
      const models = await listOllamaModels(draft.base_url);
      setOllamaModels(models);
      if (models.length > 0 && !models.includes(draft.model)) {
        setDraft((prev) => ({ ...prev, model: models[0] }));
      }
    } catch (e) {
      setError(String(e));
      setOllamaModels([]);
    } finally {
      setLoadingOllama(false);
    }
  }

  async function handleSave() {
    if (apiKeyRequired && !draft.api_key.trim()) {
      setError("请填写 API Key");
      return;
    }
    if (
      (draft.provider === "open_ai_compatible" || draft.provider === "ollama") &&
      !(draft.base_url ?? "").trim()
    ) {
      setError("请填写 Base URL");
      return;
    }
    if (!draft.model.trim()) {
      setError("请填写模型名称");
      return;
    }

    setSaving(true);
    setError(null);
    try {
      await saveSettings(draft);
      setSettings(draft);
      setLoaded(true);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <div className="max-h-[90vh] w-full max-w-lg overflow-y-auto rounded-xl border border-[var(--border)] bg-[var(--bg-panel)] p-6 shadow-2xl">
        <h2 className="mb-4 text-lg font-semibold">Settings</h2>

        <div className="space-y-4">
          <label className="block space-y-1">
            <span className="text-sm text-[var(--text-muted)]">模型提供商</span>
            <select
              value={draft.preset_id}
              onChange={(e) => selectPreset(e.target.value as ModelPresetId)}
              className="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 text-sm"
            >
              {PRESET_GROUPS.map((group) => (
                <optgroup key={group.key} label={group.label}>
                  {MODEL_PRESETS.filter((p) => p.group === group.key).map(
                    (preset) => (
                      <option key={preset.id} value={preset.id}>
                        {preset.label}
                      </option>
                    ),
                  )}
                </optgroup>
              ))}
            </select>
          </label>

          {showBaseUrl && (
            <label className="block space-y-1">
              <span className="text-sm text-[var(--text-muted)]">Base URL</span>
              <input
                value={draft.base_url ?? ""}
                onChange={(e) =>
                  setDraft({ ...draft, base_url: e.target.value || null })
                }
                className="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 text-sm font-mono"
                placeholder={
                  draft.provider === "ollama"
                    ? "http://localhost:11434/v1"
                    : "https://api.example.com/v1"
                }
              />
            </label>
          )}

          <label className="block space-y-1">
            <div className="flex items-center justify-between">
              <span className="text-sm text-[var(--text-muted)]">Model</span>
              {draft.provider === "ollama" && (
                <button
                  type="button"
                  onClick={refreshOllamaModels}
                  disabled={loadingOllama}
                  className="text-xs text-[var(--accent)] hover:underline disabled:opacity-50"
                >
                  {loadingOllama ? "加载中…" : "从 Ollama 拉取模型列表"}
                </button>
              )}
            </div>
            {draft.provider === "ollama" && ollamaModels.length > 0 ? (
              <select
                value={draft.model}
                onChange={(e) =>
                  setDraft({ ...draft, model: e.target.value })
                }
                className="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 text-sm font-mono"
              >
                {ollamaModels.map((name) => (
                  <option key={name} value={name}>
                    {name}
                  </option>
                ))}
              </select>
            ) : (
              <input
                value={draft.model}
                onChange={(e) =>
                  setDraft({ ...draft, model: e.target.value })
                }
                className="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 text-sm font-mono"
                placeholder={activePreset.modelHint ?? activePreset.defaultModel}
              />
            )}
            {activePreset.modelHint && (
              <p className="text-xs text-[var(--text-muted)]">
                例如：{activePreset.modelHint}
              </p>
            )}
          </label>

          <label className="block space-y-1">
            <span className="text-sm text-[var(--text-muted)]">
              API Key{apiKeyRequired ? "" : "（可选）"}
            </span>
            <input
              type="password"
              value={draft.api_key}
              onChange={(e) =>
                setDraft({ ...draft, api_key: e.target.value })
              }
              className="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 text-sm font-mono"
              placeholder={activePreset.apiKeyPlaceholder ?? "sk-..."}
            />
          </label>

          {draft.provider === "ollama" && (
            <p className="rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 text-xs leading-relaxed text-[var(--text-muted)]">
              请确保 Ollama 已启动（<code className="font-mono">ollama serve</code>
              ）。本地调用通常无需 API Key。
            </p>
          )}
        </div>

        {error && (
          <p className="mt-3 text-sm text-[var(--danger)]">{error}</p>
        )}

        <div className="mt-6 flex justify-end gap-2">
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

export async function loadSettingsOnBoot() {
  const { setSettings, setLoaded } = useSettingsStore.getState();
  try {
    const settings = await getSettings();
    setSettings({
      ...settings,
      preset_id:
        settings.preset_id ?? inferPresetFromSettings(settings).id,
    });
  } finally {
    setLoaded(true);
  }
}
