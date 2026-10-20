// A file in the side panel is read, saved, watched and handed on only when a source names it: a Session's
// ledger, settings.json, or the dialog the person chose it in. A path from the renderer alone is nothing.

import type { SessionRecord } from "@armada/protocol";
import { createHash } from "node:crypto";
import { chmod, mkdtemp, readdir, readFile, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { PanelFileItem } from "../shared/api/files";
import { createFilePanel, diskPanel, type PanelDisk } from "./file-panel";

const sha = (bytes: Uint8Array | string): string => createHash("sha256").update(bytes).digest("hex");
const bytesOf = (text: string): Uint8Array => new TextEncoder().encode(text);

const record = {
  id: "S1",
  cwd: "/repo",
  attachments: [
    { kind: "artifact", target: "/repo/docs/spike.md", state: "standing", detail: { form: "file" } },
    { kind: "artifact", target: "notes/todo.txt", state: "standing", detail: { form: "file" } },
    { kind: "artifact", target: "/tmp/shot.png", state: "standing", detail: { form: "image" } },
  ],
} as unknown as SessionRecord;

/** A disk in memory. `touch` is the OS telling a watcher something under a directory changed. */
function memory(files: Record<string, string>) {
  const held = new Map<string, Uint8Array>(Object.entries(files).map(([path, text]) => [path, bytesOf(text)]));
  const watchers = new Map<string, Set<(name: string | null) => void>>();
  const closed: string[] = [];
  const reads: string[] = [];
  const replaced: string[] = [];
  const created: string[] = [];
  const disk: PanelDisk = {
    size: async (path) => {
      const one = held.get(path);
      if (one === undefined) throw new Error("ENOENT");
      return one.byteLength;
    },
    read: async (path) => {
      reads.push(path);
      const one = held.get(path);
      if (one === undefined) throw new Error("ENOENT");
      return one;
    },
    replace: async (path, bytes) => {
      replaced.push(path);
      held.set(path, bytes);
    },
    create: async (path) => {
      created.push(path);
      if (!held.has(path)) held.set(path, bytesOf("{}\n"));
    },
    watch: (dir, onEvent) => {
      const set = watchers.get(dir) ?? new Set();
      set.add(onEvent);
      watchers.set(dir, set);
      return { close: () => void (set.delete(onEvent), closed.push(dir)) };
    },
  };
  const touch = (dir: string, name: string | null) => watchers.get(dir)?.forEach((one) => one(name));
  return { disk, held, replaced, created, reads, closed, touch, watching: (dir: string) => watchers.get(dir)?.size ?? 0 };
}

function wired(files: Record<string, string>, extra: { settings?: string; pick?: string | null } = {}) {
  const on = memory(files);
  const sent: Array<[unknown, PanelFileItem]> = [];
  const inEditor = vi.fn(async (_path: string): Promise<string | null> => null);
  const showInFolder = vi.fn((_path: string) => {});
  const panel = createFilePanel({
    disk: on.disk,
    recordOf: (id) => (id === "S1" ? record : undefined),
    settingsPath: () => extra.settings,
    pick: async () => extra.pick ?? null,
    inEditor,
    showInFolder,
    send: (owner, item) => void sent.push([owner, item]),
    settleMs: 50,
  });
  return { ...on, panel, sent, inEditor, showInFolder };
}

const session = (path: string, sessionId = "S1"): PanelFileItem => ({ kind: "file", path, from: "session", sessionId });
const settings = (path: string): PanelFileItem => ({ kind: "file", path, from: "settings" });
const picked = (path: string): PanelFileItem => ({ kind: "file", path, from: "picked" });
const kit = (path: string): PanelFileItem => ({ kind: "file", path, from: "kit" });

describe("a path no source names", () => {
  const files = { "/repo/docs/spike.md": "# spike", "/repo/notes/todo.txt": "todo", "/etc/passwd": "root", "/home/user/settings.json": "{}", "/home/user/other.txt": "x" };

  it("is refused for each source, and a named one reads", async () => {
    const { panel, reads } = wired(files, { settings: "/home/user/settings.json" });
    const owner = {};
    expect(await panel.read(owner, session("/etc/passwd"))).toEqual({ ok: false, why: "not_addressable" });
    expect(await panel.read(owner, session("/repo/docs/spike.md", "S9"))).toEqual({ ok: false, why: "not_addressable" });
    expect(await panel.read(owner, { kind: "file", path: "/repo/docs/spike.md", from: "session" })).toEqual({ ok: false, why: "not_addressable" });
    expect(await panel.read(owner, settings("/home/user/other.txt"))).toEqual({ ok: false, why: "not_addressable" });
    expect(await panel.read(owner, kit("/repo/docs/spike.md"))).toEqual({ ok: false, why: "not_addressable" });
    expect(await panel.read(owner, picked("/etc/passwd"))).toEqual({ ok: false, why: "not_addressable" });
    expect(reads).toEqual([]);

    expect(await panel.read(owner, session("/repo/docs/spike.md"))).toEqual({ ok: true, text: "# spike", type: "text/markdown", version: sha("# spike") });
    expect(await panel.read(owner, settings("/home/user/settings.json"))).toMatchObject({ ok: true, text: "{}" });
  });

  it("reads a Session's relative target against the Session's folder, and only as the ledger wrote it", async () => {
    const { panel } = wired(files);
    expect(await panel.read({}, session("notes/todo.txt"))).toMatchObject({ ok: true, text: "todo" });
    // The same file by its absolute path is not what the ledger names.
    expect(await panel.read({}, session("/repo/notes/todo.txt"))).toEqual({ ok: false, why: "not_addressable" });
  });

  it("is refused for a picked file until the dialog has chosen it, and reads after", async () => {
    const { panel } = wired(files, { pick: "/home/user/other.txt" });
    expect(await panel.read({}, picked("/home/user/other.txt"))).toEqual({ ok: false, why: "not_addressable" });
    expect(await panel.pick()).toEqual(picked("/home/user/other.txt"));
    expect(await panel.read({}, picked("/home/user/other.txt"))).toMatchObject({ ok: true, text: "x" });
    expect(await panel.read({}, picked("/etc/passwd"))).toEqual({ ok: false, why: "not_addressable" });
  });

  it("answers null from a dialog that was closed, and picks nothing", async () => {
    const { panel } = wired(files, { pick: null });
    expect(await panel.pick()).toBeNull();
  });

  it("creates settings.json with nothing in it where the machine has none, as opening it does", async () => {
    const { panel, created } = wired({}, { settings: "/home/user/settings.json" });
    expect(await panel.read({}, settings("/home/user/settings.json"))).toMatchObject({ ok: true, text: "{}\n" });
    expect(created).toEqual(["/home/user/settings.json"]);
  });

  it("reads a picture as bytes and refuses binary and big files", async () => {
    const png = wired({ "/tmp/shot.png": "png!" });
    expect(await png.panel.read({}, session("/tmp/shot.png"))).toMatchObject({ ok: true, bytes: bytesOf("png!"), type: "image/png" });
    const bin = wired({ "/repo/docs/spike.md": "a\u0000b" });
    expect(await bin.panel.read({}, session("/repo/docs/spike.md"))).toEqual({ ok: false, why: "binary" });
  });
});

describe("saving", () => {
  const path = "/repo/docs/spike.md";

  it("refuses a stale version as moved, with what is on disk, and writes nothing", async () => {
    const { panel, held, replaced } = wired({ [path]: "theirs" });
    const saved = await panel.save(session(path), "mine", sha("what I read"));
    expect(saved).toEqual({ state: "moved", onDisk: "theirs", version: sha("theirs") });
    expect(replaced).toEqual([]);
    expect(held.get(path)).toEqual(bytesOf("theirs"));
  });

  it("writes at the current version and answers the new one", async () => {
    const { panel, held } = wired({ [path]: "before" });
    expect(await panel.save(session(path), "after", sha("before"))).toEqual({ state: "saved", version: sha("after") });
    expect(held.get(path)).toEqual(bytesOf("after"));
  });

  it("overwrites when the moved answer's version comes back with the save", async () => {
    const { panel, held } = wired({ [path]: "theirs" });
    const moved = await panel.save(session(path), "mine", sha("what I read"));
    if (moved.state !== "moved") throw new Error("expected moved");
    expect(await panel.save(session(path), "mine", moved.version)).toEqual({ state: "saved", version: sha("mine") });
    expect(held.get(path)).toEqual(bytesOf("mine"));
  });

  it("refuses to write a path no source names", async () => {
    const { panel, replaced } = wired({ "/etc/passwd": "root" });
    expect(await panel.save(session("/etc/passwd"), "x", sha("root"))).toMatchObject({ state: "failed" });
    expect(await panel.save(picked("/etc/passwd"), "x", sha("root"))).toMatchObject({ state: "failed" });
    expect(replaced).toEqual([]);
  });

  it("says the OS's own words when the write is refused", async () => {
    const { panel, disk } = wired({ [path]: "before" });
    disk.replace = async () => {
      throw new Error("EACCES: permission denied, open '/repo/docs/.spike.md.tmp'");
    };
    expect(await panel.save(session(path), "after", sha("before"))).toEqual({
      state: "failed",
      detail: "EACCES: permission denied, open '/repo/docs/.spike.md.tmp'",
    });
  });
});

describe("opening in another program", () => {
  it("refuses a path no source names, and starts nothing", async () => {
    const { panel, inEditor, showInFolder } = wired({});
    expect(await panel.openIn(session("/etc/passwd"), "editor")).toMatchObject({ ok: false });
    expect(await panel.openIn(picked("/etc/passwd"), "finder")).toMatchObject({ ok: false });
    expect(await panel.openIn(kit("/etc/passwd"), "finder")).toMatchObject({ ok: false });
    expect(inEditor).not.toHaveBeenCalled();
    expect(showInFolder).not.toHaveBeenCalled();
  });

  it("opens a named path in the editor or shows it in Finder, a relative one by where it is", async () => {
    const { panel, inEditor, showInFolder } = wired({});
    expect(await panel.openIn(session("notes/todo.txt"), "editor")).toEqual({ ok: true });
    expect(inEditor).toHaveBeenCalledWith("/repo/notes/todo.txt");
    expect(await panel.openIn(session("/repo/docs/spike.md"), "finder")).toEqual({ ok: true });
    expect(showInFolder).toHaveBeenCalledWith("/repo/docs/spike.md");
  });

  it("carries the editor's own reason when it will not open", async () => {
    const { panel, inEditor } = wired({});
    inEditor.mockResolvedValueOnce("editor.command names code, and nothing on PATH is called that.");
    expect(await panel.openIn(session("/repo/docs/spike.md"), "editor")).toEqual({
      ok: false,
      detail: "editor.command names code, and nothing on PATH is called that.",
    });
  });
});

describe("watching the open file", () => {
  afterEach(() => vi.useRealTimers());
  const path = "/repo/docs/spike.md";

  it("tells the window that read it, once, after a burst settles", async () => {
    vi.useFakeTimers();
    const { panel, sent, held, touch } = wired({ [path]: "one" });
    const owner = { window: 1 };
    await panel.read(owner, session(path));
    held.set(path, bytesOf("two"));
    touch("/repo/docs", "spike.md");
    touch("/repo/docs", "spike.md");
    touch("/repo/docs", "other.md");
    await vi.advanceTimersByTimeAsync(10);
    expect(sent).toEqual([]);
    await vi.advanceTimersByTimeAsync(100);
    expect(sent).toEqual([[owner, session(path)]]);
  });

  it("says nothing of a touch that left the bytes as they were, nor of its own save", async () => {
    vi.useFakeTimers();
    const { panel, sent, touch } = wired({ [path]: "one" });
    const owner = {};
    await panel.read(owner, session(path));
    touch("/repo/docs", "spike.md");
    await vi.advanceTimersByTimeAsync(100);
    expect(sent).toEqual([]);
    await panel.save(session(path), "two", sha("one"), owner);
    touch("/repo/docs", "spike.md");
    await vi.advanceTimersByTimeAsync(100);
    expect(sent).toEqual([]);
  });

  it("holds one watcher per window: reading another file lets go of the first, and a window going lets go of its own", async () => {
    const { panel, watching, closed } = wired({ [path]: "one", "/repo/notes/todo.txt": "todo" });
    const owner = {};
    await panel.read(owner, session(path));
    await panel.read(owner, session(path));
    expect(watching("/repo/docs")).toBe(1);
    await panel.read(owner, session("notes/todo.txt"));
    expect(watching("/repo/docs")).toBe(0);
    expect(watching("/repo/notes")).toBe(1);
    panel.release(owner);
    expect(watching("/repo/notes")).toBe(0);
    expect(closed).toEqual(["/repo/docs", "/repo/notes"]);
  });

  it("watches nothing for a read that was refused", async () => {
    const { panel, watching } = wired({ "/etc/passwd": "root" });
    await panel.read({}, session("/etc/passwd"));
    expect(watching("/etc")).toBe(0);
  });
});

describe("on a real disk", () => {
  it("replaces the file whole, keeps its mode, and leaves no temporary file behind", async () => {
    const dir = await mkdtemp(join(tmpdir(), "file-panel-"));
    const path = join(dir, "a.txt");
    await writeFile(path, "before");
    await chmod(path, 0o640);
    await diskPanel.replace(path, bytesOf("after"));
    expect(await readFile(path, "utf8")).toBe("after");
    expect((await stat(path)).mode & 0o777).toBe(0o640);
    expect(await readdir(dir)).toEqual(["a.txt"]);
  });
});
