// A shared sketch written to disk. **The renderer sends bytes and a file name; the place is the
// person's, chosen in a save dialog main opens**, so nothing the renderer says names a path. The kinds
// are three and fixed, and a file written always ends in one of the extensions the dialog offered.

import { writeFile } from "node:fs/promises";
import { basename, extname, join } from "node:path";
import { app, BrowserWindow, dialog } from "electron";
import type { IpcMain } from "electron";

import type { SketchSaveFile, SketchSaveKind, SketchSaved } from "../shared/api/sketch-share";
import { CHANNELS } from "../shared/bridge";
import { printToPdf, saveDocumentFile } from "./document-save";

/** A picture of a big sketch at 2x is a few MB; far past this is not a sketch. */
export const MOST_BYTES = 64 * 1024 * 1024;

const KINDS: Record<SketchSaveKind, { extension: string; label: string }> = {
  html: { extension: "html", label: "Web page" },
  png: { extension: "png", label: "PNG image" },
  svg: { extension: "svg", label: "SVG image" },
};

export type SaveHost = {
  /** The place the person chose, or `undefined` where they closed the dialog. */
  choose: (options: { defaultName: string; filters: { name: string; extensions: string[] }[] }) => Promise<string | undefined>;
  write: (path: string, bytes: Uint8Array) => Promise<void>;
};

/** A name with no folder in it, and one the file system takes. */
// eslint-disable-next-line no-control-regex -- control characters are what is being taken out
const plain = (name: string) => basename(name.replace(/[\\/]+/g, "/")).replace(/[\u0000-\u001f\u007f:*?"<>|]+/g, " ").trim();

type Offered = SketchSaveFile & { extension: string };

/** The files as main will offer them, or `undefined` where the request is not one it takes. */
function accepted(files: unknown): Offered[] | undefined {
  if (!Array.isArray(files) || files.length === 0 || files.length > Object.keys(KINDS).length) return undefined;
  const out: Offered[] = [];
  for (const one of files as unknown[]) {
    if (typeof one !== "object" || one === null) return undefined;
    const { kind, name, bytes } = one as Record<string, unknown>;
    if (typeof kind !== "string" || !(kind in KINDS) || typeof name !== "string" || !(bytes instanceof Uint8Array)) return undefined;
    if (bytes.byteLength === 0 || bytes.byteLength > MOST_BYTES || out.some((seen) => seen.kind === kind)) return undefined;
    const { extension } = KINDS[kind as SketchSaveKind];
    const clean = plain(name);
    const file = clean === "" ? `Sketch.${extension}` : extname(clean).toLowerCase() === `.${extension}` ? clean : `${clean}.${extension}`;
    out.push({ kind: kind as SketchSaveKind, bytes, extension, name: file });
  }
  return out;
}

export async function saveSketchFile(files: unknown, host: SaveHost): Promise<SketchSaved> {
  const offered = accepted(files);
  if (offered === undefined) return { saved: false, why: "failed" };
  const first = offered[0]!;
  const chosen = await host.choose({
    defaultName: first.name,
    filters: offered.map((one) => ({ name: KINDS[one.kind].label, extensions: [one.extension] })),
  });
  if (chosen === undefined) return { saved: false, why: "cancelled" };
  // The extension they left on decides the format; one they typed over is put back to the first offered.
  const type = extname(chosen).slice(1).toLowerCase();
  const file = offered.find((one) => one.extension === type);
  const path = file === undefined ? `${chosen}.${first.extension}` : chosen;
  try {
    await host.write(path, (file ?? first).bytes);
    return { saved: true, name: basename(path) };
  } catch {
    return { saved: false, why: "failed" };
  }
}

export function handleSketchShare({ ipc }: { ipc: IpcMain }): void {
  ipc.handle(CHANNELS.saveSketchFile, (event, files: unknown) => {
    const window = BrowserWindow.fromWebContents(event.sender);
    return saveSketchFile(files, {
      choose: async ({ defaultName, filters }) => {
        const options = { defaultPath: join(app.getPath("downloads"), defaultName), filters };
        const chosen = window === null ? await dialog.showSaveDialog(options) : await dialog.showSaveDialog(window, options);
        return chosen.canceled || chosen.filePath === "" ? undefined : chosen.filePath;
      },
      write: (path, bytes) => writeFile(path, bytes),
    });
  });
  ipc.handle(CHANNELS.saveDocument, (event, request: unknown) => {
    const window = BrowserWindow.fromWebContents(event.sender);
    return saveDocumentFile(request, {
      choose: async ({ defaultName, filters }) => {
        const options = { defaultPath: join(app.getPath("downloads"), defaultName), filters };
        const chosen = window === null ? await dialog.showSaveDialog(options) : await dialog.showSaveDialog(window, options);
        return chosen.canceled || chosen.filePath === "" ? undefined : chosen.filePath;
      },
      write: (path, bytes) => writeFile(path, bytes),
      print: printToPdf,
    });
  });
}
