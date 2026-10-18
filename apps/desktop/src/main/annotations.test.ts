import { mkdtemp, mkdir, readdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import type { IncomingMessage, ServerResponse } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Readable } from "node:stream";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
  annotationId,
  isAnnotation,
  isAnnotationId,
  serializeAnnotation,
  ANNOTATION_CHANNELS,
  type Annotation,
} from "../shared/annotations";
import { handleAnnotations, listAnnotations, readShot, removeAnnotation, repositoryRoot, saveAnnotation, saveShot } from "./annotations";
import { answer } from "./annotations-server";

function note(overrides: Partial<Annotation> = {}): Annotation {
  return {
    id: "20260916-222105-k3f9",
    status: "open",
    text: "The legend is illegible against the row tint",
    component: "JobRowStacked",
    owners: ["ActiveJobsList", "Board"],
    ownersFrom: "parent",
    selector: '#root > div.armada-shell > ul > li:nth-of-type(2)',
    element: { tag: "li", text: "Fix the flaky test", label: null },
    screen: "Job Board",
    layer: null,
    location: "/index.html",
    scenario: null,
    box: { x: 12, y: 80, width: 900, height: 64 },
    window: { width: 1280, height: 800 },
    createdAt: "2026-09-16T22:21:05.000Z",
    updatedAt: "2026-09-16T22:21:05.000Z",
    ...overrides,
  };
}

describe("the note", () => {
  it("takes a sortable id from the time it was written", () => {
    expect(annotationId(new Date("2026-09-16T22:21:05.123Z"), () => 0)).toBe("20260916-222105-0000");
    expect(isAnnotationId(annotationId(new Date(), Math.random))).toBe(true);
  });

  it("refuses an id that could leave the directory", () => {
    for (const id of ["../x", "a/b", "", "a.json", "x".repeat(81)]) expect(isAnnotationId(id)).toBe(false);
  });

  it("is refused with a field missing or mistyped", () => {
    expect(isAnnotation(note())).toBe(true);
    expect(isAnnotation({ ...note(), status: "closed" })).toBe(false);
    expect(isAnnotation({ ...note(), owners: [1] })).toBe(false);
    const { box: _box, ...boxless } = note();
    expect(isAnnotation(boxless)).toBe(false);
  });

  it("serializes with the text and the component first, and parses back", () => {
    const text = serializeAnnotation({ ...note(), updatedAt: "later" });
    expect(Object.keys(JSON.parse(text)).slice(0, 4)).toEqual(["id", "status", "text", "component"]);
    expect(JSON.parse(serializeAnnotation(note()))).toEqual(note());
    expect(text.endsWith("}\n")).toBe(true);
  });
});

describe("what the note read of the page around it", () => {
  it("keeps nearby and selected when read, and a note written before them still reads", () => {
    const read = note({ nearby: ["Burn by Bash command", "Usage"], selected: ['tab "Bash"'] });
    expect(isAnnotation(read)).toBe(true);
    expect(JSON.parse(serializeAnnotation(read))).toEqual(read);
    expect(Object.keys(JSON.parse(serializeAnnotation(note())))).not.toContain("nearby");
    expect(isAnnotation({ ...note(), nearby: [1] })).toBe(false);
    expect(isAnnotation({ ...note(), selected: "tab" })).toBe(false);
  });
});

describe("the note's picture", () => {
  it("is named on the note as its own id's PNG, and a note without one still reads", () => {
    const shot = note({ screenshot: `${note().id}.png` });
    expect(isAnnotation(shot)).toBe(true);
    expect(JSON.parse(serializeAnnotation(shot))).toEqual(shot);
    expect(Object.keys(JSON.parse(serializeAnnotation(note())))).not.toContain("screenshot");
    for (const screenshot of ["other.png", "../x.png", `${note().id}.jpg`, 1]) expect(isAnnotation({ ...note(), screenshot })).toBe(false);
  });
});

/** The eight bytes a PNG starts with, and one more. */
const PNG = Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10, 0]);

describe("the files", () => {
  let dir = "";
  beforeEach(async () => {
    dir = join(await mkdtemp(join(tmpdir(), "annotations-")), ".armada", "annotations");
  });
  afterEach(async () => {
    await rm(join(dir, "..", ".."), { recursive: true, force: true });
  });

  it("writes one file per note, named by its id, and reads them back oldest first", async () => {
    await saveAnnotation(dir, note({ id: "b", createdAt: "2026-09-16T23:00:00.000Z" }));
    await saveAnnotation(dir, note({ id: "a" }));
    expect((await readdir(dir)).sort()).toEqual(["a.json", "b.json"]);
    expect((await listAnnotations(dir)).map((n) => n.id)).toEqual(["a", "b"]);
  });

  it("overwrites a note on a second save, and deletes it", async () => {
    await saveAnnotation(dir, note());
    await saveAnnotation(dir, note({ status: "done" }));
    expect((await listAnnotations(dir))[0]?.status).toBe("done");
    await removeAnnotation(dir, note().id);
    expect(await listAnnotations(dir)).toEqual([]);
  });

  // What the owner saw on 1 Oct 2026: two saves of one note at once shared one
  // `.partial`, the first rename moved it, and the second failed with ENOENT.
  it("takes two saves of one note at once, and leaves one whole note and no partial", async () => {
    await mkdir(dir, { recursive: true });
    await Promise.all([saveAnnotation(dir, note({ text: "first" })), saveAnnotation(dir, note({ text: "second" }))]);
    expect(await readdir(dir)).toEqual([`${note().id}.json`]);
    const saved: unknown = JSON.parse(await readFile(join(dir, `${note().id}.json`), "utf8"));
    expect(isAnnotation(saved)).toBe(true);
    expect(["first", "second"]).toContain((saved as Annotation).text);
  });

  it("never reads a write a crash left half done as a note", async () => {
    await saveAnnotation(dir, note());
    await writeFile(join(dir, `${note({ id: "b" }).id}.json.0f3a.partial`), serializeAnnotation(note({ id: "b" })));
    await writeFile(join(dir, `${note().id}.json.partial`), "{");
    expect((await listAnnotations(dir)).map((n) => n.id)).toEqual([note().id]);
  });

  it("writes nothing for a value that is not a note", async () => {
    await expect(saveAnnotation(dir, { id: "x" })).rejects.toThrow();
    await expect(removeAnnotation(dir, "../../etc")).rejects.toThrow();
  });

  it("skips a file it cannot read rather than dropping every pin", async () => {
    await saveAnnotation(dir, note());
    await writeFile(join(dir, "broken.json"), "{");
    await writeFile(join(dir, "renamed.json"), serializeAnnotation(note()));
    expect((await listAnnotations(dir)).map((n) => n.id)).toEqual([note().id]);
  });

  it("keeps a note's picture beside it, reads it back, and never lists it as a note", async () => {
    await saveAnnotation(dir, note({ screenshot: `${note().id}.png` }));
    await saveShot(dir, note().id, PNG.buffer);
    expect((await readdir(dir)).sort()).toEqual([`${note().id}.json`, `${note().id}.png`]);
    expect(new Uint8Array((await readShot(dir, note().id))!)).toEqual(PNG);
    expect((await listAnnotations(dir)).map((n) => n.id)).toEqual([note().id]);
    expect(await readShot(dir, "absent")).toBeNull();
    // Not even a `.png` holding a note's text, which only a hand could write.
    await writeFile(join(dir, "20260916-222106-zzzz.png"), serializeAnnotation(note({ id: "20260916-222106-zzzz" })));
    expect((await listAnnotations(dir)).map((n) => n.id)).toEqual([note().id]);
  });

  it("deletes the picture with the note", async () => {
    await saveAnnotation(dir, note({ screenshot: `${note().id}.png` }));
    await saveShot(dir, note().id, PNG);
    await removeAnnotation(dir, note().id);
    expect(await readdir(dir)).toEqual([]);
  });

  it("writes a picture through a rename, so a reader holds the old file or the new one and never half of one", async () => {
    await saveShot(dir, note().id, PNG);
    const before = (await stat(join(dir, `${note().id}.png`))).ino;
    await saveShot(dir, note().id, PNG);
    // A rename lands a new file where the old one was; a write in place keeps the inode.
    expect((await stat(join(dir, `${note().id}.png`))).ino).not.toBe(before);
    const other = Uint8Array.from([...PNG, 1, 2, 3]);
    await Promise.all([saveShot(dir, note().id, PNG), saveShot(dir, note().id, other)]);
    expect(await readdir(dir)).toEqual([`${note().id}.png`]);
    expect([PNG.length, other.length]).toContain((await readFile(join(dir, `${note().id}.png`))).length);
  });

  it("writes no picture that is not a PNG, or under an id that could leave the directory", async () => {
    await expect(saveShot(dir, note().id, new TextEncoder().encode("<svg/>"))).rejects.toThrow("not a PNG");
    await expect(saveShot(dir, note().id, "iVBORw0KGgo=")).rejects.toThrow("not a PNG");
    await expect(saveShot(dir, "../../etc", PNG)).rejects.toThrow("not an annotation id");
    await expect(readShot(dir, "../x")).rejects.toThrow("not an annotation id");
    expect(await listAnnotations(dir)).toEqual([]);
    await expect(readdir(dir)).rejects.toThrow();
  });

  it("answers the picture channels from main, refusing what is not an id or a PNG", async () => {
    const root = join(dir, "..", "..");
    await writeFile(join(root, "pnpm-workspace.yaml"), "");
    await writeFile(join(root, "Cargo.toml"), "");
    const handlers = new Map<string, (event: unknown, ...args: unknown[]) => unknown>();
    expect(handleAnnotations({ handle: (channel, listener) => void handlers.set(channel, listener) }, root, async () => null)).toBe(dir);
    const call = (channel: string, ...args: unknown[]) => Promise.resolve(handlers.get(channel)!({}, ...args));

    await expect(call(ANNOTATION_CHANNELS.saveShot, "../x", PNG)).rejects.toThrow();
    await expect(call(ANNOTATION_CHANNELS.saveShot, note().id, { length: 9 })).rejects.toThrow();
    await call(ANNOTATION_CHANNELS.saveShot, note().id, PNG.buffer);
    expect(new Uint8Array((await call(ANNOTATION_CHANNELS.shot, note().id)) as ArrayBuffer)).toEqual(PNG);
    await call(ANNOTATION_CHANNELS.remove, note().id);
    expect(await call(ANNOTATION_CHANNELS.shot, note().id)).toBeNull();
  });

  it("lists nothing when the directory does not exist yet", async () => {
    expect(await listAnnotations(join(dir, "absent"))).toEqual([]);
  });

  it("finds the repository root above a nested path", async () => {
    const root = join(dir, "..", "..");
    await writeFile(join(root, "pnpm-workspace.yaml"), "");
    await writeFile(join(root, "Cargo.toml"), "");
    await mkdir(join(root, "apps", "desktop"), { recursive: true });
    expect(repositoryRoot(join(root, "apps", "desktop"))).toBe(root);
  });
});

/** A request as connect hands it to the middleware, with the mount path stripped. */
function request(method: string, url: string, body?: string | Uint8Array): IncomingMessage {
  const stream = Readable.from(body === undefined ? [] : [Buffer.from(body)]) as unknown as IncomingMessage;
  return Object.assign(stream, { method, url });
}

function response(): ServerResponse & { status: () => number; body: () => string } {
  let status = 0;
  let body = "";
  const res = {
    set statusCode(value: number) {
      status = value;
    },
    setHeader: () => undefined,
    end: (chunk?: string | Buffer) => {
      body = typeof chunk === "string" ? chunk : chunk === undefined ? "" : chunk.toString("latin1");
    },
    status: () => status,
    body: () => body,
  };
  return res as unknown as ServerResponse & { status: () => number; body: () => string };
}

describe("the dev server's routes", () => {
  let dir = "";
  beforeEach(async () => {
    dir = await mkdtemp(join(tmpdir(), "annotations-server-"));
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("saves on PUT, lists on GET and deletes on DELETE", async () => {
    const put = response();
    await answer(dir, request("PUT", `/${note().id}`, JSON.stringify(note())), put);
    expect(put.status()).toBe(204);
    expect(JSON.parse(await readFile(join(dir, `${note().id}.json`), "utf8"))).toEqual(note());

    const get = response();
    await answer(dir, request("GET", "/"), get);
    expect(JSON.parse(get.body())).toEqual([note()]);

    const del = response();
    await answer(dir, request("DELETE", `/${note().id}`), del);
    expect(del.status()).toBe(204);
    expect(await readdir(dir)).toEqual([]);
  });

  it("keeps a note's picture on PUT, serves it on GET, and deletes it with the note", async () => {
    await answer(dir, request("PUT", `/${note().id}`, JSON.stringify(note({ screenshot: `${note().id}.png` }))), response());
    const put = response();
    await answer(dir, request("PUT", `/${note().id}.png`, PNG), put);
    expect(put.status()).toBe(204);

    const get = response();
    await answer(dir, request("GET", `/${note().id}.png`), get);
    expect(get.status()).toBe(200);
    expect(Buffer.from(get.body(), "latin1")).toEqual(Buffer.from(PNG));

    const list = response();
    await answer(dir, request("GET", "/"), list);
    expect(JSON.parse(list.body()).map((one: Annotation) => one.id)).toEqual([note().id]);

    await answer(dir, request("DELETE", `/${note().id}`), response());
    expect(await readdir(dir)).toEqual([]);
    const gone = response();
    await answer(dir, request("GET", `/${note().id}.png`), gone);
    expect(gone.status()).toBe(404);
  });

  it("refuses a picture that is not a PNG", async () => {
    const res = response();
    await answer(dir, request("PUT", `/${note().id}.png`, "<svg/>"), res);
    expect(res.status()).toBe(400);
    expect(await readdir(dir)).toEqual([]);
  });

  it("refuses a body whose id is not the path's", async () => {
    const res = response();
    await answer(dir, request("PUT", "/other", JSON.stringify(note())), res);
    expect(res.status()).toBe(400);
    expect(await readdir(dir)).toEqual([]);
  });
});
