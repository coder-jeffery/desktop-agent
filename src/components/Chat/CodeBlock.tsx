import { useMemo } from "react";
import hljs from "highlight.js/lib/core";
import rust from "highlight.js/lib/languages/rust";
import typescript from "highlight.js/lib/languages/typescript";
import javascript from "highlight.js/lib/languages/javascript";
import python from "highlight.js/lib/languages/python";
import go from "highlight.js/lib/languages/go";
import java from "highlight.js/lib/languages/java";
import kotlin from "highlight.js/lib/languages/kotlin";
import swift from "highlight.js/lib/languages/swift";
import c from "highlight.js/lib/languages/c";
import cpp from "highlight.js/lib/languages/cpp";
import csharp from "highlight.js/lib/languages/csharp";
import ruby from "highlight.js/lib/languages/ruby";
import php from "highlight.js/lib/languages/php";
import bash from "highlight.js/lib/languages/bash";
import powershell from "highlight.js/lib/languages/powershell";
import sql from "highlight.js/lib/languages/sql";
import xml from "highlight.js/lib/languages/xml";
import css from "highlight.js/lib/languages/css";
import scss from "highlight.js/lib/languages/scss";
import less from "highlight.js/lib/languages/less";
import json from "highlight.js/lib/languages/json";
import yaml from "highlight.js/lib/languages/yaml";
import ini from "highlight.js/lib/languages/ini";
import markdown from "highlight.js/lib/languages/markdown";
import diff from "highlight.js/lib/languages/diff";
import dockerfile from "highlight.js/lib/languages/dockerfile";
import makefile from "highlight.js/lib/languages/makefile";
import graphql from "highlight.js/lib/languages/graphql";
import r from "highlight.js/lib/languages/r";
import lua from "highlight.js/lib/languages/lua";
import dart from "highlight.js/lib/languages/dart";
import scala from "highlight.js/lib/languages/scala";
import {
  languageFileFormat,
  languageFromPath,
  languageLabel,
  normalizeLanguage,
} from "../../lib/language";

let registered = false;
function ensureLanguages() {
  if (registered) return;
  registered = true;
  const langs: Array<[string, typeof rust]> = [
    ["rust", rust],
    ["typescript", typescript],
    ["javascript", javascript],
    ["python", python],
    ["go", go],
    ["java", java],
    ["kotlin", kotlin],
    ["swift", swift],
    ["c", c],
    ["cpp", cpp],
    ["csharp", csharp],
    ["ruby", ruby],
    ["php", php],
    ["bash", bash],
    ["powershell", powershell],
    ["sql", sql],
    ["xml", xml],
    ["css", css],
    ["scss", scss],
    ["less", less],
    ["json", json],
    ["yaml", yaml],
    ["ini", ini],
    ["markdown", markdown],
    ["diff", diff],
    ["dockerfile", dockerfile],
    ["makefile", makefile],
    ["graphql", graphql],
    ["r", r],
    ["lua", lua],
    ["dart", dart],
    ["scala", scala],
  ];
  for (const [name, def] of langs) {
    hljs.registerLanguage(name, def);
  }
}

interface CodeBlockProps {
  code: string;
  language?: string | null;
  /** Absolute or relative path — drives language + format badge. */
  path?: string | null;
  maxHeightClass?: string;
}

export function CodeBlock({
  code,
  language,
  path,
  maxHeightClass = "max-h-96",
}: CodeBlockProps) {
  ensureLanguages();

  const lang = normalizeLanguage(language ?? (path ? languageFromPath(path) : null));
  const label = languageLabel(lang);
  const format = languageFileFormat(lang, path);

  const html = useMemo(() => {
    const source = code.replace(/\n$/, "");
    try {
      if (lang !== "plaintext" && hljs.getLanguage(lang)) {
        return hljs.highlight(source, { language: lang }).value;
      }
      return hljs.highlightAuto(source).value;
    } catch {
      return escapeHtml(source);
    }
  }, [code, lang]);

  return (
    <div className="my-2 overflow-hidden rounded-lg border border-[var(--border)] bg-[var(--bg)]">
      <div className="flex items-center justify-between gap-2 border-b border-[var(--border)] bg-[var(--bg-panel)] px-3 py-1.5">
        <div className="flex min-w-0 items-center gap-2 text-[11px]">
          <span className="font-medium text-[var(--text)]">{label}</span>
          <span className="truncate font-mono text-[var(--text-muted)]">
            {format}
          </span>
        </div>
      </div>
      <pre
        className={`${maxHeightClass} overflow-auto p-3 font-mono text-[12px] leading-relaxed`}
      >
        <code
          className={`hljs language-${lang}`}
          dangerouslySetInnerHTML={{ __html: html }}
        />
      </pre>
    </div>
  );
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}
