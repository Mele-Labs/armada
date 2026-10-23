// A file in the side panel: read, saved, watched and handed to another program.
//
// **The renderer names a panel item, never a path of its own, and main touches `item.path` only when the source
// the item says it came from names it.** A Session's ledger names the files it wrote or looked at, settings.json
// is the one path Fleet reported, and a picked file is one the person chose in the open dialog this run. `kit` is
// a path a fresh Kit inventory names (`kit-names.ts`), asked of Fleet each time, as `openKitFile` does.
// Electron stays out of this file; `file-panel-channels.ts` hands in the dialog, the editor and Finder.

import type { KitInventory, SessionRecord } from "@armada/protocol";
import { createHash, randomBytes } from "node:crypto";
import { watch } from "node:fs";
import { chmod, open, readFile, realpath, rename, stat, unlink, writeFile } from "node:fs/promises";
import { basename, dirname, isAbsolute, join, resolve, sep } from "node:path";

import type { PanelFileItem, PanelFileOpenedIn, PanelFileRead, PanelFileSaved } from "../shared/api/files";
import { namesKitPath } from "./kit-names";
import { MOST_BYTES, namesFile, typeOfFile } from "./session-file";
import type { FileReads } from "./session-file";

/** What the panel needs of a disk beyond reading: write a file whole, make settings.json, hear a directory change. */
export type PanelDisk = FileReads & {
  /** Where `path` is with its links followed, and whether it is a file; null where nothing is there. */
  real: (path: string) => Promise<{ path: string; file: boolean } | null>;
  /** Atomic: a temporary file beside the target, then a rename, so a reader sees the old file or the new and never half. */
  replace: (path: string, bytes: Uint8Array) => Promise<void>;
  /** `{}\n` where nothing is, as opening settings.json does. A file already there is left alone. */
  create: (path: string) => Promise<void>;
  /** Names the directory's entry that changed, or null where the OS did not say. */
  watch: (dir: string, onEvent: (name: string | null) => void) => { close: () => void };
};

export type PanelHosts = {
  disk: PanelDisk;
  recordOf: (sessionId: string) => SessionRecord | undefined;
  settingsPath: () => string | undefined;
  /** A fresh Kit inventory for the window `owner`, or undefined where Fleet did not answer. */
  kitInventory: (owner: unknown) => Promise<KitInventory | undefined>;
  /** The open dialog: the path chosen, or null where it was closed. */
  pick: () => Promise<string | null>;
  /** `null` is opened; a string is why not. */
  inEditor: (path: string) => Promise<string | null>;
  showInFolder: (path: string) => void;
  /** A change to the file `owner` read. */
  send: (owner: unknown, item: PanelFileItem) => void;
  /** How long a burst of events is left to settle. */
  settleMs?: number;
};

/** Where a hosted Job's checkout sits under its repository root. */
const SLOTS = `${sep}.armada${sep}slots${sep}`;

const NOT_OPENED = "Not a file this panel opened";

/** The most paths one question about a thread looks at, so a long thread is not a long walk of the disk. */
const MOST_ASKED = 200;

export const diskPanel: PanelDisk = {
  size: async (path) => (await stat(path)).size,
  read: async (path) => new Uint8Array(await readFile(path)),
  real: async (path) => {
    try {
      return { path: await realpath(path), file: (await stat(path)).isFile() };
    } catch {
      return null;
    }
  },
  replace: async (path, bytes) => {
    // The target itself, so a link to a file is written through and not replaced by one.
    const target = await realpath(path).catch(() => path);
    const mode = (await stat(target).catch(() => null))?.mode;
    const beside = join(dirname(target), `.${basename(target)}.${randomBytes(4).toString("hex")}.tmp`);
    try {
      const handle = await open(beside, "wx", 0o600);
      try {
        await handle.writeFile(bytes);
      } finally {
        await handle.close();
      }
      if (mode !== undefined) await chmod(beside, mode & 0o7777);
      await rename(beside, target);
    } catch (error) {
      await unlink(beside).catch(() => {});
      throw error;
    }
  },
  create: async (path) => {
    // Already there is the usual case; a directory that cannot be written shows when the file is read.
    await writeFile(path, "{}\n", { flag: "wx" }).catch(() => {});
  },
  watch: (dir, onEvent) => {
    try {
      const watcher = watch(dir, { persistent: false }, (_event, name) => onEvent(name === null ? null : String(name)));
      watcher.on("error", () => {});
      return { close: () => watcher.close() };
    } catch {
      return { close: () => {} };
    }
  },
};

const sha256 = (bytes: Uint8Array): string => createHash("sha256").update(bytes).digest("hex");
const sayWhy = (error: unknown): string => (error instanceof Error ? error.message : String(error));

type Watch = { disk: string; item: PanelFileItem; version: string; closer: { close: () => void }; timer: ReturnType<typeof setTimeout> | undefined };

export function createFilePanel(hosts: PanelHosts) {
  const { disk } = hosts;
  const settleMs = hosts.settleMs ?? 100;
  /** Paths the person chose in the open dialog this run. Added to by the dialog and nothing else. */
  const chosen = new Set<string>();
  /**
   * **One watcher per window**, on the directory of the file it last read and filtered to that name: a
   * directory survives an editor that saves by renaming a new file over the old, which a watch on the file
   * itself does not. Reading another file, or the window going (`release`), lets go of it.
   */
  const watches = new Map<unknown, Watch>();

  /**
   * The file `path` names inside the Session's checkout, links followed, or null. **A link that leaves the checkout
   * is outside it**: the check is on where the path ends up, not on how it was written. A relative path that misses
   * from the checkout is tried from the repository root, which a slot's cwd names (`<root>/.armada/slots/<slot>`):
   * an agent writes paths from there.
   */
  async function inCheckout(record: SessionRecord, path: string): Promise<string | null> {
    const root = await disk.real(record.cwd);
    if (root === null) return null;
    const fromHere = isAbsolute(path) ? [path] : [resolve(record.cwd, path)];
    const at = record.cwd.indexOf(SLOTS);
    if (!isAbsolute(path) && at > 0) fromHere.push(resolve(record.cwd.slice(0, at), path));
    for (const candidate of fromHere) {
      const there = await disk.real(candidate);
      if (there?.file === true && there.path.startsWith(root.path + sep)) return there.path;
    }
    return null;
  }

  /** Where on disk the item is, or null where its source does not name it. */
  async function locate(item: PanelFileItem, owner?: unknown): Promise<string | null> {
    if (typeof item?.path !== "string" || item.path === "") return null;
    switch (item.from) {
      case "session": {
        const record = typeof item.sessionId === "string" ? hosts.recordOf(item.sessionId) : undefined;
        if (record === undefined) return null;
        if (namesFile(record, item.path)) return isAbsolute(item.path) ? item.path : resolve(record.cwd, item.path);
        return inCheckout(record, item.path); // the same file `known` offered the link for
      }
      case "settings":
        return item.path === hosts.settingsPath() ? item.path : null;
      case "picked":
        return chosen.has(item.path) ? item.path : null;
      case "kit": {
        const setup = await hosts.kitInventory(owner);
        return setup !== undefined && namesKitPath(setup, item.path) ? item.path : null;
      }
      default:
        return null;
    }
  }

  function release(owner: unknown): void {
    const one = watches.get(owner);
    if (one === undefined) return;
    clearTimeout(one.timer);
    one.closer.close();
    watches.delete(owner);
  }

  function keepWatching(owner: unknown, item: PanelFileItem, path: string, version: string): void {
    const held = watches.get(owner);
    if (held?.disk === path) {
      held.item = item;
      held.version = version;
      return;
    }
    release(owner);
    const name = basename(path);
    const one: Watch = { disk: path, item, version, timer: undefined, closer: { close: () => {} } };
    // Only a change in the bytes is told: a touch, an atomic rename that left them alone, and the window's own save are not.
    const check = async (): Promise<void> => {
      const now = await disk.read(path).then(sha256, () => "gone");
      if (now === one.version || watches.get(owner) !== one) return;
      one.version = now;
      hosts.send(owner, one.item);
    };
    one.closer = disk.watch(dirname(path), (changed) => {
      if (changed !== null && changed !== name) return;
      clearTimeout(one.timer);
      one.timer = setTimeout(() => void check(), settleMs);
    });
    watches.set(owner, one);
  }

  return {
    async read(owner: unknown, item: PanelFileItem): Promise<PanelFileRead> {
      const path = await locate(item, owner);
      if (path === null) return { ok: false, why: "not_addressable" };
      try {
        if (item.from === "settings") await disk.create(path);
        if ((await disk.size(path)) > MOST_BYTES) return { ok: false, why: "too_big" };
        const bytes = await disk.read(path);
        const type = typeOfFile(path);
        const version = sha256(bytes);
        if (type.startsWith("image/")) {
          keepWatching(owner, item, path, version);
          return { ok: true, bytes, type, version };
        }
        if (bytes.subarray(0, 4096).includes(0)) return { ok: false, why: "binary" };
        // Strict, so a file that is not UTF-8 is refused rather than saved back with its bytes replaced.
        const text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
        keepWatching(owner, item, path, version);
        return { ok: true, text, type, version };
      } catch (error) {
        return error instanceof TypeError ? { ok: false, why: "binary" } : { ok: false, why: "unreadable" };
      }
    },

    /**
     * **Writes only over the version the window read.** The file is read again here and a different hash
     * means something else wrote it: nothing is written and what is there comes back. A save that passes the
     * version of that answer is Keep mine. `owner`, where given, is not told of the write it made itself.
     */
    async save(item: PanelFileItem, text: string, version: string, owner?: unknown): Promise<PanelFileSaved> {
      const path = await locate(item, owner);
      if (path === null || typeOfFile(path).startsWith("image/")) return { state: "failed", detail: NOT_OPENED };
      try {
        const now = await disk.read(path);
        const current = sha256(now);
        if (current !== version) return { state: "moved", onDisk: new TextDecoder("utf-8", { ignoreBOM: true }).decode(now), version: current };
        const bytes = new TextEncoder().encode(text);
        await disk.replace(path, bytes);
        const saved = sha256(bytes);
        const own = owner === undefined ? undefined : watches.get(owner);
        if (own?.disk === path) own.version = saved;
        return { state: "saved", version: saved };
      } catch (error) {
        return { state: "failed", detail: sayWhy(error) };
      }
    },

    /** Of `paths`, those that are files in the Session's checkout, as they were written. At most `MOST_ASKED` are looked at. */
    async known(sessionId: string, paths: readonly string[]): Promise<string[]> {
      const record = hosts.recordOf(sessionId);
      if (record === undefined) return [];
      const found = await Promise.all(paths.slice(0, MOST_ASKED).map(async (path) => (typeof path === "string" && path !== "" && (await inCheckout(record, path)) !== null ? path : null)));
      return found.filter((one): one is string => one !== null);
    },

    async pick(): Promise<PanelFileItem | null> {
      const path = await hosts.pick();
      if (path === null) return null;
      chosen.add(path);
      return { kind: "file", path, from: "picked" };
    },

    async openIn(item: PanelFileItem, where: "editor" | "finder", owner?: unknown): Promise<PanelFileOpenedIn> {
      const path = await locate(item, owner);
      if (path === null) return { ok: false, detail: NOT_OPENED };
      if (where === "finder") {
        hosts.showInFolder(path);
        return { ok: true };
      }
      if (item.from === "settings") await disk.create(path);
      const refused = await hosts.inEditor(path);
      return refused === null ? { ok: true } : { ok: false, detail: refused };
    },

    release,
  };
}
