import type { Repository } from "./types";

export type RepoTreeNode =
  | {
      kind: "folder";
      id: string;
      name: string;
      children: RepoTreeNode[];
    }
  | {
      kind: "repo";
      id: string;
      name: string;
      path: string;
      repoId: string;
    };

type MutableFolder = {
  kind: "folder";
  id: string;
  name: string;
  children: Map<string, MutableNode>;
};

type MutableRepo = {
  kind: "repo";
  id: string;
  name: string;
  path: string;
  repoId: string;
};

type MutableNode = MutableFolder | MutableRepo;

function normalizePath(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "");
}

function pathSegments(path: string): string[] {
  return normalizePath(path).split("/").filter(Boolean);
}

function commonPrefixLength(paths: string[][]): number {
  if (paths.length === 0) return 0;
  let depth = paths[0].length;
  for (const segs of paths) {
    depth = Math.min(depth, segs.length);
  }
  // Keep at least the leaf segment for each repo
  depth = Math.max(0, depth - 1);
  for (let i = 0; i < depth; i++) {
    const head = paths[0][i];
    if (!paths.every((p) => p[i] === head)) {
      return i;
    }
  }
  return depth;
}

function toTree(node: MutableNode): RepoTreeNode {
  if (node.kind === "repo") {
    return {
      kind: "repo",
      id: node.id,
      name: node.name,
      path: node.path,
      repoId: node.repoId,
    };
  }
  const children = [...node.children.values()]
    .map(toTree)
    .sort((a, b) => {
      if (a.kind !== b.kind) return a.kind === "folder" ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
  return {
    kind: "folder",
    id: node.id,
    name: node.name,
    children,
  };
}

/** Build a collapsible folder tree from repository absolute paths. */
export function buildRepoTree(repos: Repository[]): RepoTreeNode[] {
  if (repos.length === 0) return [];

  const allSegments = repos.map((r) => pathSegments(r.path));
  const skip = commonPrefixLength(allSegments);

  const root: MutableFolder = {
    kind: "folder",
    id: "root",
    name: "",
    children: new Map(),
  };

  for (const repo of repos) {
    const segs = pathSegments(repo.path).slice(skip);
    if (segs.length === 0) {
      segs.push(repo.name);
    }

    let current = root;
    let pathId = "root";

    for (let i = 0; i < segs.length; i++) {
      const name = segs[i];
      const isLeaf = i === segs.length - 1;
      pathId = `${pathId}/${name}`;

      if (isLeaf) {
        current.children.set(name, {
          kind: "repo",
          id: `repo:${repo.id}`,
          name: repo.name || name,
          path: repo.path,
          repoId: repo.id,
        });
        break;
      }

      let next = current.children.get(name);
      if (!next || next.kind !== "folder") {
        next = {
          kind: "folder",
          id: `folder:${pathId}`,
          name,
          children: new Map(),
        };
        current.children.set(name, next);
      }
      current = next;
    }
  }

  return [...root.children.values()]
    .map(toTree)
    .sort((a, b) => {
      if (a.kind !== b.kind) return a.kind === "folder" ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
}

/** Collect folder ids that should stay open so the active repo remains visible. */
export function folderIdsToReveal(
  nodes: RepoTreeNode[],
  activeRepoId: string | null,
): string[] {
  if (!activeRepoId) return [];
  const found: string[] = [];

  function walk(list: RepoTreeNode[], trail: string[]): boolean {
    for (const node of list) {
      if (node.kind === "repo") {
        if (node.repoId === activeRepoId) {
          found.push(...trail);
          return true;
        }
      } else if (walk(node.children, [...trail, node.id])) {
        return true;
      }
    }
    return false;
  }

  walk(nodes, []);
  return found;
}
