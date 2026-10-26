// A Job's document written to disk, as a web page or as a PDF. **The renderer sends the page it built and a file name; the place is the
// person's, chosen in a save dialog main opens**, as a sketch's is (`sketch-share.ts`). The page is self-contained and carries no script.
// A PDF is that same page printed in a window that is never shown, with scripts off and nothing but the file itself loadable.

import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, extname, join } from "node:path";
import { BrowserWindow, session } from "electron";

import type { DocumentSaveKind, SketchSaved } from "../shared/api/sketch-share";

/** A page with its sketches drawn into it is a few hundred KB; far past this is not a document. */
export const MOST_CHARS = 32 * 1024 * 1024;

const KINDS: Record<DocumentSaveKind, { extension: string; label: string }> = {
  html: { extension: "html", label: "Web page" },
  pdf: { extension: "pdf", label: "PDF" },
};

export type DocumentHost = {
  /** The place the person chose, or `undefined` where they closed the dialog. */
  choose: (options: { defaultName: string; filters: { name: string; extensions: string[] }[] }) => Promise<string | undefined>;
  write: (path: string, bytes: Uint8Array) => Promise<void>;
  /** The page printed to PDF. */
  print: (html: string) => Promise<Uint8Array>;
};

/** A name with no folder in it, and one the file system takes. */
// eslint-disable-next-line no-control-regex -- control characters are what is being taken out
const plain = (name: string) => basename(name.replace(/[\\/]+/g, "/")).replace(/[\u0000-\u001f\u007f:*?"<>|]+/g, " ").trim();

export async function saveDocumentFile(request: unknown, host: DocumentHost): Promise<SketchSaved> {
  if (typeof request !== "object" || request === null) return { saved: false, why: "failed" };
  const { kind, name, html } = request as Record<string, unknown>;
  if ((kind !== "html" && kind !== "pdf") || typeof name !== "string" || typeof html !== "string" || html === "" || html.length > MOST_CHARS) return { saved: false, why: "failed" };
  const { extension, label } = KINDS[kind];
  const clean = plain(name);
  const defaultName = clean === "" ? `Document.${extension}` : extname(clean).toLowerCase() === `.${extension}` ? clean : `${clean}.${extension}`;
  const chosen = await host.choose({ defaultName, filters: [{ name: label, extensions: [extension] }] });
  if (chosen === undefined) return { saved: false, why: "cancelled" };
  const path = extname(chosen).toLowerCase() === `.${extension}` ? chosen : `${chosen}.${extension}`;
  try {
    await host.write(path, kind === "pdf" ? await host.print(html) : new TextEncoder().encode(html));
    return { saved: true, name: basename(path) };
  } catch {
    return { saved: false, why: "failed" };
  }
}

/**
 * The page printed to PDF. It is written to a file of its own and loaded from there, since an address that holds the whole page
 * is cut at a size a long document passes. The window is never shown, runs no script and can load nothing but that file.
 */
export async function printToPdf(html: string): Promise<Uint8Array> {
  const folder = await mkdtemp(join(tmpdir(), "armada-document-"));
  const file = join(folder, "page.html");
  // In memory, and the window's own: nothing it does reaches the app's session.
  const own = session.fromPartition("armada-document-pdf");
  own.webRequest.onBeforeRequest((details, answer) => answer({ cancel: !details.url.startsWith("file://") }));
  const window = new BrowserWindow({
    show: false,
    skipTaskbar: true,
    focusable: false,
    webPreferences: { session: own, javascript: false, sandbox: true, contextIsolation: true, nodeIntegration: false, webSecurity: true },
  });
  try {
    window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
    window.webContents.on("will-navigate", (event) => event.preventDefault());
    await writeFile(file, html);
    await window.loadFile(file);
    return new Uint8Array(await window.webContents.printToPDF({ printBackground: true, pageSize: "A4" }));
  } finally {
    window.destroy();
    await rm(folder, { recursive: true, force: true });
  }
}
