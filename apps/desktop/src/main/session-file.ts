// Open a file a Session wrote, from its ledger row.
//
// **The renderer names a session and a path, and main opens the path only if that session's own
// ledger names it as a file it wrote or a picture it looked at.** A string from a click handler going to `shell.openPath` would
// make every capability the sandbox holds back reachable through one row; the record main already
// holds is the set the string has to be a member of, as `open.ts` does for a Job's records.

import type { Followed, SessionRecord } from "@armada/protocol";
import type { ArtifactRead } from "@armada/screens/src/draft/sessions";
import { extname } from "node:path";

export function namesFile(record: SessionRecord | undefined, path: string): boolean {
  return (
    record?.attachments.some((one) => one.kind === "artifact" && one.target === path && (one.detail?.["form"] === "file" || one.detail?.["form"] === "image")) === true
  );
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
