// Open a file a Session wrote, from its ledger row.
//
// **The renderer names a session and a path, and main opens the path only if that session's own
// ledger names it as a file it wrote or a picture it looked at.** A string from a click handler going to `shell.openPath` would
// make every capability the sandbox holds back reachable through one row; the record main already
// holds is the set the string has to be a member of, as `open.ts` does for a Job's records.

import type { Followed, SessionRecord, WalkStep, WalkTarget, WindowTake } from "@armada/protocol";
import type { ArtifactRead } from "@armada/screens/src/draft/sessions";
import { revisionsOfDetail } from "@armada/screens/src/sessions-wire";
import { extname } from "node:path";

export function namesFile(record: SessionRecord | undefined, path: string): boolean {
  return (
    record?.attachments.some(
      (one) =>
        one.kind === "artifact" &&
        ((one.target === path && (one.detail?.["form"] === "file" || one.detail?.["form"] === "image")) ||
          // A sent note's picture, at the path Fleet recorded for it.
          (one.detail?.["form"] === "note" && one.detail["picture"] === path) ||
          // A document's pages are the revisions Fleet kept for it, and only those.
          (one.detail?.["form"] === "document" && revisionsOfDetail(one.detail["revisions"]).includes(path)) ||
          // A sketch's redraws, the same way.
          (one.detail?.["form"] === "sketch" && revisionsOfDetail(one.detail["revisions"]).includes(path))),
    ) === true
  );
}

/** The title of the window a Session showed at `url`, or `null` where its ledger shows none there. */
export function titleOfWindow(record: SessionRecord | undefined, url: string): string | null {
  const shown = record?.attachments.find(
    (one) => one.kind === "artifact" && one.target === url && one.detail?.["form"] === "window",
  );
  return shown === undefined ? null : (shown.detail?.["title"] ?? url);
}

/**
 * The takes the Session showed at `url`, as its ledger keeps them in the window's `detail`, or none.
 * **Anything that is not a list of `{name, url}` reads as none**: the ledger is Fleet's, but a
 * window opens on what it says, so each address is held to the web here as it was there.
 */
export function takesOfWindow(record: SessionRecord | undefined, url: string): WindowTake[] {
  const kept = record?.attachments.find(
    (one) => one.kind === "artifact" && one.target === url && one.detail?.["form"] === "window",
  )?.detail?.["variations"];
  if (kept === undefined) return [];
  try {
    const read: unknown = JSON.parse(kept);
    if (!Array.isArray(read)) return [];
    const takes = read.map((one: unknown) => one as Partial<WindowTake> | null);
    return takes.every((one) => typeof one?.name === "string" && typeof one.url === "string" && /^https?:\/\//.test(one.url))
      ? (takes as WindowTake[])
      : [];
  } catch {
    return [];
  }
}

const VERBS = ["press", "look", "hover", "later", "type", "key", "select", "paste", "drag"] as const;
/** The most steps a window plays, and how deep a target may nest: Fleet's own bounds, held again here. */
const MOST_STEPS = 200;
const MOST_NESTING = 4;

const record_ = (value: unknown): Record<string, unknown> | null =>
  typeof value === "object" && value !== null && !Array.isArray(value) ? (value as Record<string, unknown>) : null;

function targetOf(value: unknown, depth: number): WalkTarget | null {
  const read = record_(value);
  if (read === null || typeof read["role"] !== "string" || read["role"] === "") return null;
  const { name, exact, within } = read;
  if ((name !== undefined && typeof name !== "string") || (exact !== undefined && typeof exact !== "boolean")) return null;
  if (read["role"] === "text" && name === undefined) return null;
  let inner: WalkTarget | undefined;
  if (within !== undefined) {
    if (depth >= MOST_NESTING) return null;
    const found = targetOf(within, depth + 1);
    if (found === null) return null;
    inner = found;
  }
  return {
    role: read["role"],
    ...(name === undefined ? {} : { name: name as string }),
    ...(exact === undefined ? {} : { exact: exact as boolean }),
    ...(inner === undefined ? {} : { within: inner }),
  };
}

/**
 * The steps a window plays, from what Fleet kept, or none. **Anything that is not a list of steps
 * as `show_window` takes them reads as none**: Fleet checked them, and a window plays what it reads,
 * so each is held to the same shape here. A step is a `say` and exactly one verb with its target.
 */
export function readSteps(value: unknown): WalkStep[] {
  if (!Array.isArray(value) || value.length > MOST_STEPS) return [];
  const steps: WalkStep[] = [];
  for (const one of value) {
    const read = record_(one);
    if (read === null || typeof read["say"] !== "string" || read["say"].trim() === "") return [];
    const verbs = VERBS.filter((verb) => read[verb] !== undefined);
    const verb = verbs[0];
    if (verbs.length !== 1 || verb === undefined) return [];
    const say = read["say"];
    if (verb === "type" || verb === "key" || verb === "select") {
      const words = read[verb];
      const aimed = targetOf(read[verb === "type" ? "into" : verb === "key" ? "on" : "in"], 0);
      if (typeof words !== "string" || words === "" || aimed === null) return [];
      if (verb === "type") steps.push({ say, type: words, into: aimed });
      else if (verb === "key") steps.push({ say, key: words, on: aimed });
      else steps.push({ say, select: words, in: aimed });
      continue;
    }
    const aimed = targetOf(read[verb], 0);
    if (aimed === null) return [];
    if (verb === "drag") {
      const by = record_(read["by"]);
      if (by === null || !Number.isFinite(by["x"]) || !Number.isFinite(by["y"])) return [];
      steps.push({ say, drag: aimed, by: { x: by["x"] as number, y: by["y"] as number } });
    } else steps.push({ say, [verb]: aimed } as WalkStep);
  }
  return steps;
}

/** The steps the Session showed at `url`, as its ledger keeps them in the window's `detail`, or none. */
export function stepsOfWindow(record: SessionRecord | undefined, url: string): WalkStep[] {
  const kept = record?.attachments.find(
    (one) => one.kind === "artifact" && one.target === url && one.detail?.["form"] === "window",
  )?.detail?.["steps"];
  if (kept === undefined) return [];
  try {
    return readSteps(JSON.parse(kept));
  } catch {
    return [];
  }
}

/**
 * The origins the Session let the window at `url` pass through, as its ledger keeps them in the
 * window's `detail`, or none. **Anything that is not a list of strings reads as none**; the window
 * holds each to a bare origin before it trusts one with a navigation (`passThrough`).
 */
export function originsOfWindow(record: SessionRecord | undefined, url: string): string[] {
  const kept = record?.attachments.find(
    (one) => one.kind === "artifact" && one.target === url && one.detail?.["form"] === "window",
  )?.detail?.["origins"];
  if (kept === undefined) return [];
  try {
    const read: unknown = JSON.parse(kept);
    return Array.isArray(read) && read.every((one) => typeof one === "string") ? (read as string[]) : [];
  } catch {
    return [];
  }
}

export async function openSessionFile(
  record: SessionRecord | undefined,
  path: string,
  // Electron is met only when a file is opened, so a test of the host that holds this never loads it.
  open: (path: string) => Promise<string> = async (one) => (await import("electron")).shell.openPath(one),
): Promise<Followed> {
  if (!namesFile(record, path)) return { ok: false, why: "not_addressable", address: path };
  const refused = await open(path);
  return refused === "" ? { ok: true } : { ok: false, why: "refused", address: path, detail: refused };
}

/** The most a panel reads of a file, so a log or a recording does not fill the renderer. */
export const MOST_BYTES = 8 * 1024 * 1024;

const PICTURES: Record<string, string> = {
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".webp": "image/webp",
  ".svg": "image/svg+xml",
};

/** What the panel draws a file as, off its name: a picture, markdown, or text. */
export function typeOfFile(path: string): string {
  const ext = extname(path).toLowerCase();
  return PICTURES[ext] ?? (ext === ".md" || ext === ".markdown" ? "text/markdown" : "text/plain");
}

/** What reads a file: its size first, so a big one is refused before it is held. */
export type FileReads = { size: (path: string) => Promise<number>; read: (path: string) => Promise<Uint8Array> };

const onDisk: FileReads = {
  size: async (path) => (await (await import("node:fs/promises")).stat(path)).size,
  read: async (path) => new Uint8Array(await (await import("node:fs/promises")).readFile(path)),
};

/**
 * A file the Session's ledger names, read for the panel. **Only a path the ledger names**, as
 * `openSessionFile` is; a page's address is refused here and shown by `session-page.ts`.
 */
export async function readSessionArtifact(record: SessionRecord | undefined, path: string, reads: FileReads = onDisk): Promise<ArtifactRead> {
  if (!namesFile(record, path)) return { ok: false, why: "not_addressable" };
  try {
    if ((await reads.size(path)) > MOST_BYTES) return { ok: false, why: "too_big", limit: MOST_BYTES };
    const bytes = await reads.read(path);
    const type = typeOfFile(path);
    if (!type.startsWith("image/") && bytes.subarray(0, 4096).includes(0)) return { ok: false, why: "binary" };
    return { ok: true, bytes, type };
  } catch {
    return { ok: false, why: "unreadable" };
  }
}

/** Whether the ledger names this address as a page or a doc, and it is one a web view may load. */
export function namesPage(record: SessionRecord | undefined, address: string): boolean {
  if (!/^https?:\/\//i.test(address)) return false;
  return record?.attachments.some((one) => one.kind === "artifact" && one.target === address && (one.detail?.["form"] === "page" || one.detail?.["form"] === "doc")) === true;
}
