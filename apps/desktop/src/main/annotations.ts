// The files the annotation layer writes, #1226. Dev only.
//
// **No Electron import, on purpose.** The vite plugin in
// `annotations-server.ts` runs this in a plain Node process beside a dev
// server, and a module that pulled in `electron` would not load there. What
// main needs from Electron it passes in.
//
// Main registers these handlers only when `app.isPackaged` is false, and the
// preload exposes `window.armadaDev` only when main passed `ANNOTATE_FLAG`. A
// packaged Bridge has neither end of the channel.

import { randomUUID } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, readdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";

import {
  ANNOTATION_CHANNELS,
  ANNOTATIONS_DIR,
  byCreation,
  isAnnotation,
  isAnnotationId,
  isExampleOf,
  isPng,
  serializeAnnotation,
  shotName,
  type Annotation,
  type Box,
} from "../shared/annotations";

/**
 * The repository root above `start`: the first directory holding both
 * `pnpm-workspace.yaml` and `Cargo.toml`. In a worktree that is the worktree,
 * so notes left on a worktree's Bridge land in that worktree.
 */
export function repositoryRoot(start: string): string | null {
  let at = resolve(start);
  for (;;) {
    if (existsSync(join(at, "pnpm-workspace.yaml")) && existsSync(join(at, "Cargo.toml"))) return at;
    const up = dirname(at);
    if (up === at) return null;
    at = up;
  }
}

/**
 * The repository Fleet serves for a checkout of it. **A worktree, the preview among them, is not
 * what Fleet lists**: its main repository is, and a send naming the worktree is refused or lands
 * nowhere. Where git cannot say, or the checkout is its own repository, it is `root` itself.
 */
export function servedRoot(root: string): string {
  try {
    const common = execFileSync("git", ["-C", root, "rev-parse", "--path-format=absolute", "--git-common-dir"], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
    return basename(common) === ".git" ? dirname(common) : root;
  } catch {
    return root;
  }
}

export function annotationsDir(root: string): string {
  return join(root, ...ANNOTATIONS_DIR);
}

/**
 * Every note in the directory, oldest first. A file that does not parse or is
 * not a note is skipped rather than failing the list — one hand-edited file
 * must not take every pin off the screen.
 */
export async function listAnnotations(dir: string): Promise<Annotation[]> {
  let names: string[];
  try {
    names = await readdir(dir);
  } catch {
    return [];
  }
  const notes: Annotation[] = [];
  for (const name of names) {
    if (!name.endsWith(".json")) continue;
    try {
      const parsed: unknown = JSON.parse(await readFile(join(dir, name), "utf8"));
      if (isAnnotation(parsed) && `${parsed.id}.json` === name) notes.push(parsed);
    } catch {
      // Unreadable or half-written; skipped, see above.
    }
  }
  return notes.sort(byCreation);
}

/**
 * Writes one note to `<id>.json`, through a rename so a reader never sees half of it.
 *
 * **The partial is named per write**, so two saves of one note at once never
 * share one and the last rename lands a whole note. One shared `<id>.json.partial`
 * failed the second rename with ENOENT on 1 Oct 2026. A partial a crash leaves
 * does not end in `.json`, so `listAnnotations` never reads it.
 */
export async function saveAnnotation(dir: string, note: unknown): Promise<void> {
  if (!isAnnotation(note)) throw new Error("not an annotation");
  await mkdir(dir, { recursive: true });
  const path = join(dir, `${note.id}.json`);
  const partial = `${path}.${randomUUID()}.partial`;
  await writeFile(partial, serializeAnnotation(note), "utf8");
  await rename(partial, path);
}

/** The note, its picture and its example pictures, where it has them. */
export async function removeAnnotation(dir: string, id: unknown): Promise<void> {
  if (!isAnnotationId(id)) throw new Error("not an annotation id");
  await rm(join(dir, `${id}.json`), { force: true });
  await rm(join(dir, shotName(id)), { force: true });
  const names = await readdir(dir).catch(() => [] as string[]);
  for (const name of names) if (isExampleOf(id, name)) await rm(join(dir, name), { force: true });
}

/** Bytes as IPC or a request hands them over: an `ArrayBuffer`, or a view of one such as a `Buffer`. */
function bytesOf(value: unknown): Uint8Array | null {
  if (value instanceof ArrayBuffer) return new Uint8Array(value);
  if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
  return null;
}

/** Writes a note's picture to `<id>.png`, through a rename as `saveAnnotation` writes the note. */
export async function saveShot(dir: string, id: unknown, png: unknown): Promise<void> {
  if (!isAnnotationId(id)) throw new Error("not an annotation id");
  await writePng(dir, shotName(id), png);
}

/** A note's picture, or null where it has none. */
export async function readShot(dir: string, id: unknown): Promise<ArrayBuffer | null> {
  if (!isAnnotationId(id)) throw new Error("not an annotation id");
  return readPng(dir, shotName(id));
}

/** Writes one of a note's example pictures to `name`, which has to be `exampleName(id, n)`. */
export async function saveExample(dir: string, id: unknown, name: unknown, png: unknown): Promise<void> {
  if (!isExampleOf(id, name)) throw new Error("not an example of this note");
  await writePng(dir, name, png);
}

/** One of a note's example pictures, or null where it is gone. */
export async function readExample(dir: string, id: unknown, name: unknown): Promise<ArrayBuffer | null> {
  if (!isExampleOf(id, name)) throw new Error("not an example of this note");
  return readPng(dir, name);
}

export async function removeExample(dir: string, id: unknown, name: unknown): Promise<void> {
  if (!isExampleOf(id, name)) throw new Error("not an example of this note");
  await rm(join(dir, name), { force: true });
}

async function writePng(dir: string, name: string, png: unknown): Promise<void> {
  const bytes = bytesOf(png);
  if (bytes === null || !isPng(bytes)) throw new Error("not a PNG");
  await mkdir(dir, { recursive: true });
  const path = join(dir, name);
  const partial = `${path}.${randomUUID()}.partial`;
  await writeFile(partial, bytes);
  await rename(partial, path);
}

async function readPng(dir: string, name: string): Promise<ArrayBuffer | null> {
  try {
    const bytes = await readFile(join(dir, name));
    return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return null;
    throw error;
  }
}

/** The part of `ipcMain` this uses, so the module needs no Electron to load. */
type Handles = {
  handle: (channel: string, listener: (event: unknown, ...args: unknown[]) => unknown) => void;
};

/** What main captures a window's area with. Passed in, so this module needs no Electron. */
export type Capture = (event: unknown, box: Box) => Promise<ArrayBuffer | null>;

/**
 * Registers the channels against the repository above `appPath`. Returns the
 * directory, or null when no repository was found — a Bridge run from
 * somewhere that is not this repository has nowhere to write, and says so.
 */
export function handleAnnotations(ipc: Handles, appPath: string, capture: Capture): string | null {
  const root = repositoryRoot(appPath);
  if (root === null) {
    console.warn(`annotations: no repository above ${appPath}; the layer cannot save`);
    return null;
  }
  const dir = annotationsDir(root);
  ipc.handle(ANNOTATION_CHANNELS.list, () => listAnnotations(dir));
  ipc.handle(ANNOTATION_CHANNELS.save, (_event, note) => saveAnnotation(dir, note));
  ipc.handle(ANNOTATION_CHANNELS.remove, (_event, id) => removeAnnotation(dir, id));
  ipc.handle(ANNOTATION_CHANNELS.root, () => servedRoot(root));
  ipc.handle(ANNOTATION_CHANNELS.capture, (event, box) => (isBoxOf(box) ? capture(event, box) : null));
  ipc.handle(ANNOTATION_CHANNELS.saveShot, (_event, id, png) => saveShot(dir, id, png));
  ipc.handle(ANNOTATION_CHANNELS.shot, (_event, id) => readShot(dir, id));
  ipc.handle(ANNOTATION_CHANNELS.saveExample, (_event, id, name, png) => saveExample(dir, id, name, png));
  ipc.handle(ANNOTATION_CHANNELS.example, (_event, id, name) => readExample(dir, id, name));
  ipc.handle(ANNOTATION_CHANNELS.removeExample, (_event, id, name) => removeExample(dir, id, name));
  return dir;
}

function isBoxOf(value: unknown): value is Box {
  if (typeof value !== "object" || value === null) return false;
  const box = value as Record<string, unknown>;
  return ["x", "y", "width", "height"].every((key) => typeof box[key] === "number" && Number.isFinite(box[key]));
}
