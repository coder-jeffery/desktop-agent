import type { ProposedEdit } from "../../lib/types";
import { applyEdit, rejectEdit } from "../../lib/tauri";
import { useChatStore } from "../../stores";
import {
  languageFileFormat,
  languageFromPath,
  languageLabel,
} from "../../lib/language";

interface DiffPreviewProps {
  edit: ProposedEdit;
}

export function DiffPreview({ edit }: DiffPreviewProps) {
  const { removePendingEdit, setActiveEdit } = useChatStore();
  const lang = languageFromPath(edit.path);
  const label = languageLabel(lang);
  const format = languageFileFormat(lang, edit.path);

  async function handleApply() {
    await applyEdit(edit.id);
    removePendingEdit(edit.id);
    setActiveEdit(null);
  }

  async function handleReject() {
    await rejectEdit(edit.id);
    removePendingEdit(edit.id);
    setActiveEdit(null);
  }

  return (
    <div className="border-t border-[var(--border)] bg-[var(--bg-panel)]">
      <div className="flex items-center justify-between border-b border-[var(--border)] px-4 py-3">
        <div>
          <div className="text-sm font-medium">{edit.summary}</div>
          <div className="mt-0.5 flex flex-wrap items-center gap-2 font-mono text-xs text-[var(--text-muted)]">
            <span>{edit.path}</span>
            <span className="rounded border border-[var(--border)] px-1.5 py-0.5 text-[10px] text-[var(--text)]">
              {label}
            </span>
            <span className="text-[10px]">{format}</span>
          </div>
        </div>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={handleReject}
            className="rounded-lg border border-[var(--border)] px-3 py-1.5 text-sm hover:bg-[var(--bg-elevated)]"
          >
            Reject
          </button>
          <button
            type="button"
            onClick={handleApply}
            className="rounded-lg bg-[var(--accent-dim)] px-3 py-1.5 text-sm font-medium text-white hover:bg-[var(--accent)]"
          >
            Apply
          </button>
        </div>
      </div>
      <pre className="max-h-64 overflow-auto p-4 font-mono text-xs leading-relaxed text-[var(--text-muted)]">
        {edit.diff.split("\n").map((line, i) => {
          let color = "text-[var(--text-muted)]";
          if (line.startsWith("+") && !line.startsWith("+++")) {
            color = "text-[var(--accent)]";
          }
          if (line.startsWith("-") && !line.startsWith("---")) {
            color = "text-[var(--danger)]";
          }
          if (line.startsWith("@@")) color = "text-[var(--warning)]";
          return (
            <div key={`${i}-${line.slice(0, 12)}`} className={color}>
              {line || " "}
            </div>
          );
        })}
      </pre>
    </div>
  );
}
