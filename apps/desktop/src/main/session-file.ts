// Open a file a Session wrote, from its ledger row.
//
// **The renderer names a session and a path, and main opens the path only if that session's own
// ledger names it as a file it wrote or a picture it looked at.** A string from a click handler going to `shell.openPath` would
// make every capability the sandbox holds back reachable through one row; the record main already
// holds is the set the string has to be a member of, as `open.ts` does for a Job's records.

import type { Followed, SessionRecord } from "@armada/protocol";

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
