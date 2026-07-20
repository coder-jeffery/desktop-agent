import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  useChatStore,
  useSessionStore,
  useSettingsStore,
  useWorkspaceStore,
} from "../../stores";
import {
  applyEdit,
  cancelAgent,
  listPendingEdits,
  sendMessage,
  toBackendHistory,
} from "../../lib/tauri";
import type { AgentStreamEvent } from "../../lib/types";
import { intensityHint, isApiKeyRequired } from "../../lib/types";
import { MessageList } from "./MessageList";
import { ComposerBar } from "./ComposerBar";

interface ChatPanelProps {
  onOpenSettings: () => void;
}

export function ChatPanel({ onOpenSettings }: ChatPanelProps) {
  const [input, setInput] = useState("");
  const [attachments, setAttachments] = useState<string[]>([]);
  const bottomRef = useRef<HTMLDivElement>(null);
  const {
    messages,
    streaming,
    activeEdit,
    addUserMessage,
    appendAssistantDelta,
    addToolCall,
    addToolResult,
    finalizeAssistantTurn,
    setStreaming,
    addPendingEdit,
    setPendingEdits,
    removePendingEdit,
    setActiveEdit,
  } = useChatStore();
  const { settings, loaded, requireApproval, intensity } = useSettingsStore();
  const { path: workspace } = useWorkspaceStore();
  const persistActive = useSessionStore((s) => s.persistActive);

  useEffect(() => {
    listPendingEdits().then(setPendingEdits).catch(console.error);
  }, [setPendingEdits]);

  useEffect(() => {
    const unlisten = listen<AgentStreamEvent>("agent://stream", (event) => {
      const payload = event.payload;
      switch (payload.type) {
        case "text_delta":
          appendAssistantDelta(payload.delta);
          break;
        case "tool_call":
          addToolCall(payload.id, payload.name, payload.arguments);
          break;
        case "tool_result":
          addToolResult(payload.id, payload.name, payload.output);
          break;
        case "proposed_edit":
          if (!useSettingsStore.getState().requireApproval) {
            applyEdit(payload.edit.id)
              .then(() => {
                removePendingEdit(payload.edit.id);
                setActiveEdit(null);
                appendAssistantDelta(
                  `\n\n✓ 已自动应用：${payload.edit.path}`,
                );
              })
              .catch((err) => {
                addPendingEdit(payload.edit);
                appendAssistantDelta(`\n\n⚠ 自动应用失败：${String(err)}`);
              });
          } else {
            addPendingEdit(payload.edit);
          }
          break;
        case "turn_complete":
          finalizeAssistantTurn();
          persistActive(useChatStore.getState().messages).catch(console.error);
          break;
        case "error":
          appendAssistantDelta(`\n\n⚠ ${payload.message}`);
          finalizeAssistantTurn();
          persistActive(useChatStore.getState().messages).catch(console.error);
          break;
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [
    appendAssistantDelta,
    addToolCall,
    addToolResult,
    addPendingEdit,
    removePendingEdit,
    setActiveEdit,
    finalizeAssistantTurn,
    persistActive,
  ]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streaming, activeEdit]);

  async function handleSubmit() {
    const text = input.trim();
    if (!text || streaming) return;
    if (!workspace) {
      alert("请先添加仓库");
      return;
    }
    if (!loaded || (isApiKeyRequired(settings) && !settings.api_key.trim())) {
      alert("请先在模型设置中配置 API Key");
      onOpenSettings();
      return;
    }

    const attachmentBlock =
      attachments.length > 0
        ? `\n\n[Attached context files]\n${attachments.map((p) => `- ${p}`).join("\n")}`
        : "";
    const modeBlock = `\n\n[Agent mode]\nApproval required: ${requireApproval ? "yes" : "no (auto-apply file edits)"}\nIntensity: ${intensity} — ${intensityHint(intensity)}`;
    const fullMessage = `${text}${attachmentBlock}${modeBlock}`;

    setInput("");
    setAttachments([]);
    const history = messages;
    addUserMessage(text);
    setStreaming(true);

    const nextMessages = [...history, { role: "user" as const, content: text }];
    persistActive(nextMessages, text).catch(console.error);

    try {
      await sendMessage({
        message: fullMessage,
        history: toBackendHistory(history),
        settings,
      });
    } catch (err) {
      appendAssistantDelta(`\n\n⚠ ${String(err)}`);
      finalizeAssistantTurn();
      persistActive(useChatStore.getState().messages).catch(console.error);
    }
  }

  async function handleStop() {
    await cancelAgent();
    finalizeAssistantTurn();
    persistActive(useChatStore.getState().messages).catch(console.error);
  }

  const canSend =
    !streaming &&
    !!workspace &&
    (!isApiKeyRequired(settings) || !!settings.api_key.trim()) &&
    input.trim().length > 0;

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-4">
        <MessageList messages={messages} streaming={streaming} />
        <div ref={bottomRef} />
      </div>

      <ComposerBar
        value={input}
        onChange={setInput}
        onSubmit={handleSubmit}
        onStop={handleStop}
        streaming={streaming}
        canSend={!!canSend}
        onOpenSettings={onOpenSettings}
        attachments={attachments}
        onAttachmentsChange={setAttachments}
      />
    </div>
  );
}
