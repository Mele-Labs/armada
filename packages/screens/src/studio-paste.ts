// What ⌘V on an open Studio puts on it — the owner's note of 1 Oct 2026 and his calls on it:
// it lands at once, an address is a Link, a path is a File, a picture is a Note carrying it,
// and any other text is a Note.
//
// **Read off the shapes a real ⌘V hands Chromium on macOS**, measured in Bridge's own Electron
// and listed in the decision `2026-10-01-a-paste-lands-at-once.md`. Finder's icon never
// reaches the page, so a picture that is only the icon beside a copied file comes down to whether
// the file is on disk — which is main's to read, never the renderer's.

import type { StudioNodeByHand } from "@armada/components";

/** The clipboard as a paste reads it: its plain text, and each file with its path on disk, or `""`. */
export type Pasted = {
  text: string;
  files: readonly { type: string; path: string }[];
};

/**
 * What a paste lands as. **A picture is named and not sent**: the Note that would carry it
 * has no words, and Fleet refuses a Note with none. `null` is nothing to land.
 */
export type Landing = StudioNodeByHand | { kind: "picture" } | null;

/** Read a paste while it is happening — a `DataTransfer` is empty once the event is over. */
export function pastedOf(clipboard: DataTransfer, pathOf: (file: File) => string): Pasted {
  return {
    text: clipboard.getData("text/plain"),
    files: Array.from(clipboard.files, (file) => ({ type: file.type, path: pathOf(file) })),
  };
}

const ADDRESS = /^https?:\/\/\S+$/i;
/** One part of a path written from the repository's root. */
const PART = /^[\w.@+-]+$/;
/** A name with an extension, so `and/or` is words and `src/main.rs` is a file. */
const NAMED = /[^.]\.[A-Za-z0-9]+$/;

/**
 * A path the owner named: absolute, under home, or from the repository's root like
 * `crates/fleet/src/briefing.rs`. **One line**, and kept as pasted — nothing here checks it is
 * there, which is Fleet's rule too.
 */
export function isPath(text: string): boolean {
  if (text.includes("\n")) return false;
  if (text.startsWith("/") || text.startsWith("~/")) return text.length > 1;
  if (/\s/.test(text) || text.includes("://")) return false;
  const parts = text.replace(/\/$/, "").split("/");
  if (parts.length < 2 || !parts.every((part) => PART.test(part))) return false;
  return text.endsWith("/") || NAMED.test(parts[parts.length - 1]!);
}

/** What the paste lands as. A file on disk first, then a picture, then the text. */
export function landingOf({ text, files }: Pasted): Landing {
  const onDisk = files.find((file) => file.path !== "");
  if (onDisk !== undefined) return { kind: "file", path: onDisk.path };
  if (files.some((file) => file.type.startsWith("image/"))) return { kind: "picture" };
  const said = text.trim();
  if (said === "") return null;
  if (ADDRESS.test(said)) return { kind: "link", address: said };
  if (isPath(said)) return { kind: "file", path: said };
  return { kind: "note", said };
}
