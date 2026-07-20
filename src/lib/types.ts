export type ProviderKind = "anthropic" | "open_ai_compatible" | "ollama";

export type ModelPresetId =
  | "anthropic"
  | "openai"
  | "deepseek"
  | "qwen"
  | "moonshot"
  | "zhipu"
  | "siliconflow"
  | "ollama"
  | "custom";

export interface AppSettings {
  provider: ProviderKind;
  /** UI preset id — drives default base_url / model when switching */
  preset_id: ModelPresetId;
  model: string;
  api_key: string;
  base_url?: string | null;
}

export type MessageRole = "system" | "user" | "assistant" | "tool";

export interface ToolCallInfo {
  id: string;
  name: string;
  arguments: string;
}

export interface ChatMessage {
  role: MessageRole;
  content: string;
  toolCalls?: ToolCallInfo[];
  toolCallId?: string;
  name?: string;
}

export interface ProposedEdit {
  id: string;
  path: string;
  old_content: string;
  new_content: string;
  diff: string;
  summary: string;
}

export type AgentStreamEvent =
  | { type: "text_delta"; delta: string }
  | { type: "tool_call"; id: string; name: string; arguments: string }
  | { type: "tool_result"; id: string; name: string; output: string }
  | { type: "proposed_edit"; edit: ProposedEdit }
  | { type: "turn_complete" }
  | { type: "error"; message: string };

export interface SendMessageRequest {
  message: string;
  history: ChatMessage[];
  settings: AppSettings;
}

export interface Repository {
  id: string;
  path: string;
  name: string;
  lastOpened: number;
}

export interface Session {
  id: string;
  title: string;
  workspaceId?: string | null;
  messages: ChatMessage[];
  createdAt: number;
  updatedAt: number;
}

export interface RulesBundle {
  globalRules: string;
  workspaceRules: string;
}

export interface ModelPreset {
  id: ModelPresetId;
  label: string;
  group: "international" | "domestic" | "local" | "custom";
  provider: ProviderKind;
  defaultModel: string;
  defaultBaseUrl?: string;
  apiKeyRequired: boolean;
  apiKeyPlaceholder?: string;
  baseUrlEditable: boolean;
  modelHint?: string;
}

export type AgentIntensity = "light" | "standard" | "thorough";

export const INTENSITY_LABELS: Record<AgentIntensity, string> = {
  light: "轻度",
  standard: "标准",
  thorough: "深入",
};

export function intensityHint(intensity: AgentIntensity): string {
  switch (intensity) {
    case "light":
      return "Prefer quick, minimal changes. Keep explanations short.";
    case "thorough":
      return "Be thorough: read broadly, consider edge cases, and explain trade-offs.";
    default:
      return "Balance speed and thoroughness.";
  }
}

export function shortModelLabel(model: string): string {
  const m = model.trim();
  if (!m) return "Model";
  if (m.length <= 18) return m;
  return `${m.slice(0, 16)}…`;
}

export const DEFAULT_PRESET_ID: ModelPresetId = "ollama";

export const MODEL_PRESETS: ModelPreset[] = [
  {
    id: "anthropic",
    label: "Anthropic Claude",
    group: "international",
    provider: "anthropic",
    defaultModel: "claude-sonnet-4-20250514",
    apiKeyRequired: true,
    apiKeyPlaceholder: "sk-ant-...",
    baseUrlEditable: false,
  },
  {
    id: "openai",
    label: "OpenAI",
    group: "international",
    provider: "open_ai_compatible",
    defaultModel: "gpt-4o-mini",
    defaultBaseUrl: "https://api.openai.com/v1",
    apiKeyRequired: true,
    apiKeyPlaceholder: "sk-...",
    baseUrlEditable: true,
  },
  {
    id: "deepseek",
    label: "DeepSeek 深度求索",
    group: "domestic",
    provider: "open_ai_compatible",
    defaultModel: "deepseek-chat",
    defaultBaseUrl: "https://api.deepseek.com/v1",
    apiKeyRequired: true,
    apiKeyPlaceholder: "sk-...",
    baseUrlEditable: true,
    modelHint: "deepseek-chat / deepseek-reasoner",
  },
  {
    id: "qwen",
    label: "通义千问 (DashScope)",
    group: "domestic",
    provider: "open_ai_compatible",
    defaultModel: "qwen-plus",
    defaultBaseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    apiKeyRequired: true,
    apiKeyPlaceholder: "sk-...",
    baseUrlEditable: true,
    modelHint: "qwen-plus / qwen-turbo / qwen-max",
  },
  {
    id: "moonshot",
    label: "Moonshot / Kimi",
    group: "domestic",
    provider: "open_ai_compatible",
    defaultModel: "moonshot-v1-8k",
    defaultBaseUrl: "https://api.moonshot.cn/v1",
    apiKeyRequired: true,
    apiKeyPlaceholder: "sk-...",
    baseUrlEditable: true,
    modelHint: "moonshot-v1-8k / moonshot-v1-32k",
  },
  {
    id: "zhipu",
    label: "智谱 GLM",
    group: "domestic",
    provider: "open_ai_compatible",
    defaultModel: "glm-4-flash",
    defaultBaseUrl: "https://open.bigmodel.cn/api/paas/v4",
    apiKeyRequired: true,
    apiKeyPlaceholder: "API Key",
    baseUrlEditable: true,
    modelHint: "glm-4-flash / glm-4-plus",
  },
  {
    id: "siliconflow",
    label: "SiliconFlow 硅基流动",
    group: "domestic",
    provider: "open_ai_compatible",
    defaultModel: "deepseek-ai/DeepSeek-V3",
    defaultBaseUrl: "https://api.siliconflow.cn/v1",
    apiKeyRequired: true,
    apiKeyPlaceholder: "sk-...",
    baseUrlEditable: true,
    modelHint: "deepseek-ai/DeepSeek-V3 / Qwen/Qwen2.5-72B-Instruct",
  },
  {
    id: "ollama",
    label: "Ollama (本地)",
    group: "local",
    provider: "ollama",
    defaultModel: "llama3.2",
    defaultBaseUrl: "http://localhost:11434/v1",
    apiKeyRequired: false,
    apiKeyPlaceholder: "可选，本地通常留空",
    baseUrlEditable: true,
    modelHint: "llama3.2 / qwen2.5-coder / deepseek-r1",
  },
  {
    id: "custom",
    label: "自定义 OpenAI 兼容",
    group: "custom",
    provider: "open_ai_compatible",
    defaultModel: "",
    defaultBaseUrl: "",
    apiKeyRequired: true,
    apiKeyPlaceholder: "sk-...",
    baseUrlEditable: true,
  },
];

export function getPresetById(id: ModelPresetId): ModelPreset {
  return (
    MODEL_PRESETS.find((p) => p.id === id) ??
    MODEL_PRESETS.find((p) => p.id === DEFAULT_PRESET_ID)!
  );
}

export function inferPresetFromSettings(settings: AppSettings): ModelPreset {
  const byId = MODEL_PRESETS.find((p) => p.id === settings.preset_id);
  if (byId) return byId;

  if (settings.provider === "anthropic") return getPresetById("anthropic");
  if (settings.provider === "ollama") return getPresetById("ollama");

  const base = (settings.base_url ?? "").replace(/\/$/, "");
  const matched = MODEL_PRESETS.find(
    (p) => p.defaultBaseUrl && p.defaultBaseUrl.replace(/\/$/, "") === base,
  );
  return matched ?? getPresetById("custom");
}

export function applyPreset(presetId: ModelPresetId): AppSettings {
  const preset = getPresetById(presetId);
  return {
    provider: preset.provider,
    preset_id: preset.id,
    model: preset.defaultModel,
    base_url: preset.defaultBaseUrl ?? null,
    api_key: "",
  };
}

export function isApiKeyRequired(settings: AppSettings): boolean {
  const preset = inferPresetFromSettings(settings);
  return preset.apiKeyRequired;
}

export function getPresetLabel(settings: AppSettings): string {
  return inferPresetFromSettings(settings).label;
}

/** @deprecated use getPresetLabel */
export const PROVIDER_PRESETS: Record<
  ProviderKind,
  { label: string; defaultModel: string; defaultBaseUrl?: string }
> = {
  anthropic: {
    label: "Anthropic",
    defaultModel: "claude-sonnet-4-20250514",
  },
  open_ai_compatible: {
    label: "OpenAI Compatible",
    defaultModel: "gpt-4o-mini",
    defaultBaseUrl: "https://api.openai.com/v1",
  },
  ollama: {
    label: "Ollama",
    defaultModel: "llama3.2",
    defaultBaseUrl: "http://localhost:11434/v1",
  },
};

export const PRESET_GROUPS: { key: ModelPreset["group"]; label: string }[] = [
  { key: "international", label: "国际" },
  { key: "domestic", label: "国产" },
  { key: "local", label: "本地" },
  { key: "custom", label: "自定义" },
];
