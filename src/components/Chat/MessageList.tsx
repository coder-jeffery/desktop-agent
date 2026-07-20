import type { ChatMessage } from "../../lib/types";
import { languageFromPath } from "../../lib/language";
import { CodeBlock } from "./CodeBlock";
import { MarkdownContent } from "./MarkdownContent";

function ToolCallCard({
  name,
  argumentsJson,
}: {
  name: string;
  argumentsJson: string;
}) {
  let path: string | null = null;
  let fileContent: string | null = null;
  let preview = argumentsJson;

  try {
    const parsed = JSON.parse(argumentsJson) as Record<string, unknown>;
    if (typeof parsed.path === "string") path = parsed.path;
    if (typeof parsed.content === "string") fileContent = parsed.content;
    preview = JSON.stringify(parsed, null, 2);
  } catch {
    // keep raw
  }

  if (
    (name === "write_file" || name === "create_file") &&
    path &&
    fileContent !== null
  ) {
    return (
      <div className="mt-2 rounded-lg border border-[var(--border)] bg-[var(--bg)] p-3">
        <div className="mb-1 font-mono text-xs text-[var(--warning)]">{name}</div>
        <CodeBlock
          code={fileContent}
          path={path}
          language={languageFromPath(path)}
          maxHeightClass="max-h-64"
        />
      </div>
    );
  }

  return (
    <div className="mt-2 rounded-lg border border-[var(--border)] bg-[var(--bg)] p-3">
      <div className="mb-1 font-mono text-xs text-[var(--warning)]">{name}</div>
      <CodeBlock code={preview} language="json" maxHeightClass="max-h-32" />
    </div>
  );
}

function MessageBubble({ message }: { message: ChatMessage }) {
  if (message.role === "user") {
    return (
      <div className="flex justify-end">
        <div className="max-w-[85%] rounded-2xl rounded-tr-sm bg-[#1f6feb] px-4 py-2.5 text-sm leading-relaxed text-white whitespace-pre-wrap">
          {message.content}
        </div>
      </div>
    );
  }

  if (message.role === "tool") {
    const looksLikeJson =
      message.content.trimStart().startsWith("{") ||
      message.content.trimStart().startsWith("[");
    const language =
      message.name === "grep" || message.name === "git_diff"
        ? "plaintext"
        : looksLikeJson
          ? "json"
          : "plaintext";

    return (
      <div className="rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-4 py-3">
        <div className="mb-1 font-mono text-xs text-[var(--text-muted)]">
          tool · {message.name}
        </div>
        <CodeBlock
          code={message.content}
          language={language}
          maxHeightClass="max-h-48"
        />
      </div>
    );
  }

  return (
    <div className="rounded-2xl rounded-tl-sm border border-[var(--border)] bg-[var(--bg-elevated)] px-4 py-3">
      {message.content && <MarkdownContent content={message.content} />}
      {message.toolCalls?.map((tc) => (
        <ToolCallCard
          key={tc.id}
          name={tc.name}
          argumentsJson={tc.arguments}
        />
      ))}
    </div>
  );
}

interface MessageListProps {
  messages: ChatMessage[];
  streaming: boolean;
}

export function MessageList({ messages, streaming }: MessageListProps) {
  if (messages.length === 0) {
    return (
      <div className="flex h-full flex-col items-center justify-center px-8 text-center">
        <h2 className="mb-2 text-xl font-semibold tracking-tight">
          Desktop Agent
        </h2>
        <p className="max-w-md text-sm text-[var(--text-muted)]">
          在下方随心输入。上下文条会显示当前仓库、本地环境与 Git 分支；
          「请求批准」开启时，文件改动需确认后应用。
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {messages.map((msg, idx) => (
        <MessageBubble
          key={`${idx}-${msg.role}-${msg.content.slice(0, 20)}`}
          message={msg}
        />
      ))}
      {streaming && (
        <div className="flex items-center gap-2 text-xs text-[var(--text-muted)]">
          <span className="inline-block h-2 w-2 animate-pulse rounded-full bg-[var(--accent)]" />
          Agent is working…
        </div>
      )}
    </div>
  );
}
