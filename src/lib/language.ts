/** Map file extensions / fence tags → highlight.js language ids. */
const EXT_TO_LANG: Record<string, string> = {
  rs: "rust",
  ts: "typescript",
  tsx: "typescript",
  js: "javascript",
  jsx: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  py: "python",
  go: "go",
  java: "java",
  kt: "kotlin",
  swift: "swift",
  c: "c",
  h: "c",
  cpp: "cpp",
  cc: "cpp",
  cxx: "cpp",
  hpp: "cpp",
  cs: "csharp",
  rb: "ruby",
  php: "php",
  sh: "bash",
  bash: "bash",
  zsh: "bash",
  fish: "bash",
  ps1: "powershell",
  sql: "sql",
  html: "xml",
  htm: "xml",
  xml: "xml",
  svg: "xml",
  css: "css",
  scss: "scss",
  less: "less",
  json: "json",
  jsonc: "json",
  yaml: "yaml",
  yml: "yaml",
  toml: "ini",
  ini: "ini",
  md: "markdown",
  markdown: "markdown",
  txt: "plaintext",
  text: "plaintext",
  diff: "diff",
  patch: "diff",
  dockerfile: "dockerfile",
  makefile: "makefile",
  mk: "makefile",
  vue: "xml",
  svelte: "xml",
  graphql: "graphql",
  gql: "graphql",
  r: "r",
  lua: "lua",
  zig: "rust",
  dart: "dart",
  scala: "scala",
};

const LANG_ALIASES: Record<string, string> = {
  ts: "typescript",
  tsx: "typescript",
  js: "javascript",
  jsx: "javascript",
  py: "python",
  rb: "ruby",
  sh: "bash",
  shell: "bash",
  zsh: "bash",
  yml: "yaml",
  md: "markdown",
  rs: "rust",
  csharp: "csharp",
  "c#": "csharp",
  "c++": "cpp",
  plaintext: "plaintext",
  text: "plaintext",
  plain: "plaintext",
};

const LANG_LABELS: Record<string, string> = {
  rust: "Rust",
  typescript: "TypeScript",
  javascript: "JavaScript",
  python: "Python",
  go: "Go",
  java: "Java",
  kotlin: "Kotlin",
  swift: "Swift",
  c: "C",
  cpp: "C++",
  csharp: "C#",
  ruby: "Ruby",
  php: "PHP",
  bash: "Shell",
  powershell: "PowerShell",
  sql: "SQL",
  xml: "HTML/XML",
  css: "CSS",
  scss: "SCSS",
  less: "Less",
  json: "JSON",
  yaml: "YAML",
  ini: "TOML/INI",
  markdown: "Markdown",
  plaintext: "Plain text",
  diff: "Diff",
  dockerfile: "Dockerfile",
  makefile: "Makefile",
  graphql: "GraphQL",
  r: "R",
  lua: "Lua",
  dart: "Dart",
  scala: "Scala",
};

const LANG_TO_EXT: Record<string, string> = {
  rust: ".rs",
  typescript: ".ts",
  javascript: ".js",
  python: ".py",
  go: ".go",
  java: ".java",
  kotlin: ".kt",
  swift: ".swift",
  c: ".c",
  cpp: ".cpp",
  csharp: ".cs",
  ruby: ".rb",
  php: ".php",
  bash: ".sh",
  powershell: ".ps1",
  sql: ".sql",
  xml: ".html",
  css: ".css",
  scss: ".scss",
  less: ".less",
  json: ".json",
  yaml: ".yaml",
  ini: ".toml",
  markdown: ".md",
  plaintext: ".txt",
  diff: ".diff",
  dockerfile: "Dockerfile",
  makefile: "Makefile",
  graphql: ".graphql",
  r: ".r",
  lua: ".lua",
  dart: ".dart",
  scala: ".scala",
};

export function normalizeLanguage(raw?: string | null): string {
  if (!raw) return "plaintext";
  const key = raw.trim().toLowerCase().replace(/^\./, "");
  return LANG_ALIASES[key] ?? EXT_TO_LANG[key] ?? (key || "plaintext");
}

export function languageFromPath(path: string): string {
  const base = path.split(/[/\\]/).pop() ?? path;
  const lower = base.toLowerCase();
  if (lower === "dockerfile" || lower.startsWith("dockerfile.")) {
    return "dockerfile";
  }
  if (lower === "makefile" || lower === "gnumakefile") {
    return "makefile";
  }
  const dot = lower.lastIndexOf(".");
  if (dot < 0) return "plaintext";
  const ext = lower.slice(dot + 1);
  return EXT_TO_LANG[ext] ?? "plaintext";
}

export function languageLabel(lang: string): string {
  const id = normalizeLanguage(lang);
  return LANG_LABELS[id] ?? id;
}

export function languageFileFormat(lang: string, path?: string | null): string {
  if (path) {
    const base = path.split(/[/\\]/).pop();
    if (base) return base;
  }
  const id = normalizeLanguage(lang);
  return LANG_TO_EXT[id] ?? id;
}
