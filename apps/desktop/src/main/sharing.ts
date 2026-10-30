// A piece committed to a Job's branch, a package made, read and added, as Fleet answers `/repositories/commit` and `/share/*`, and a
// package written where the person chooses. Nothing is kept here: Fleet makes and reads the text, and main only writes the file.

import { writeFile } from "node:fs/promises";
import { basename, join } from "node:path";
import { app, BrowserWindow, dialog } from "electron";
import type { IpcMain } from "electron";

import type { AddPackage, BindUnboundPiece, CommitToRepository, Outcome, PieceScope, PieceToCommit, SharePieces } from "@armada/protocol";
import type { PackageSaved, SharingAnswer } from "../shared/api/sharing";
import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };
const unsent = { ok: false, outcome: NOT_CONNECTED } as const;

/** What every package file ends in: Fleet's `SharedPackage.file_name` does. */
export const PACKAGE_EXTENSION = ".armada.toml";

/** A package is a few tables of text; far past this is not one. */
export const MOST_PACKAGE_BYTES = 1024 * 1024;

export class SharingHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  commit(body: CommitToRepository) {
    return this.send("POST", "/repositories/commit", body);
  }

  share(body: SharePieces) {
    return this.send("POST", "/share/package", body);
  }

  read(text: string) {
    return this.send("POST", "/share/read", { text });
  }

  add(body: AddPackage) {
    return this.send("POST", "/share/add", body);
  }

  unbound() {
    return this.send("GET", "/share/unbound");
  }

  bind(body: BindUnboundPiece) {
    return this.send("POST", "/share/bind", body);
  }

  private async send<T>(method: "GET" | "POST", path: string, body?: unknown): Promise<SharingAnswer<T>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, path, body);
    return answer.ok === true ? { ok: true, value: answer.body as T } : { ok: false, outcome: answer.outcome };
  }
}

const named = (value: unknown): value is string => typeof value === "string" && value.trim() !== "";
const ids = (value: unknown): value is string[] | undefined => value === undefined || (Array.isArray(value) && value.every(named));

/** A scope as the wire spells it, or `null`. Absent is everywhere. */
const scopeOf = (scope: unknown): PieceScope | undefined | null => {
  if (scope === undefined) return undefined;
  if (typeof scope !== "object" || scope === null) return null;
  const { kind, id } = scope as Record<string, unknown>;
  if (kind === "everywhere") return { kind };
  return kind === "within" && named(id) ? { kind, id } : null;
};

/** One piece's selector, or `null` for anything else. What it names is Fleet's to refuse. */
export function pieceOf(piece: unknown): PieceToCommit | null {
  if (typeof piece !== "object" || piece === null) return null;
  const { kind, job, deck, workflow_id, id, scope } = piece as Record<string, unknown>;
  if (kind === "flight_controls" && ids(job) && ids(deck)) return { kind, ...(job === undefined ? {} : { job }), ...(deck === undefined ? {} : { deck }) };
  if (kind === "workflow" && named(workflow_id)) return { kind, workflow_id };
  if (kind === "review_tool" && named(id)) return { kind, id };
  if (kind === "manifest") return { kind };
  if (kind === "workspace" && named(id)) return { kind, id };
  const at = scopeOf(scope);
  if ((kind === "lifecycle" || kind === "lifecycle_rule" || kind === "autonomy") && named(id) && at !== null) return { kind, id, ...(at === undefined ? {} : { scope: at }) };
  return null;
}

export function commitOf(body: unknown): CommitToRepository | null {
  if (typeof body !== "object" || body === null) return null;
  const { repository, piece, overwrite, remove } = body as Record<string, unknown>;
  const chosen = pieceOf(piece);
  if (!named(repository) || chosen === null) return null;
  if (overwrite !== undefined && typeof overwrite !== "boolean") return null;
  if (remove !== undefined && typeof remove !== "boolean") return null;
  // No Job is named: Fleet starts one for the commit, and refuses a body that names one. `remove` is sent only when true: a Fleet older than it refuses the field.
  return { repository, piece: chosen, ...(overwrite === undefined ? {} : { overwrite }), ...(remove === true ? { remove } : {}) };
}

export function shareOf(body: unknown): SharePieces | null {
  if (typeof body !== "object" || body === null) return null;
  const { pieces } = body as Record<string, unknown>;
  if (!Array.isArray(pieces) || pieces.length === 0) return null;
  const out: SharePieces["pieces"] = [];
  for (const one of pieces as unknown[]) {
    if (typeof one !== "object" || one === null) return null;
    const { repository, piece } = one as Record<string, unknown>;
    const chosen = pieceOf(piece);
    if (!named(repository) || chosen === null) return null;
    out.push({ repository, piece: chosen });
  }
  return { pieces: out };
}

export function addOf(body: unknown): AddPackage | null {
  if (typeof body !== "object" || body === null) return null;
  const { text, replace } = body as Record<string, unknown>;
  if (!named(text) || (replace !== undefined && typeof replace !== "boolean")) return null;
  return { text, ...(replace === undefined ? {} : { replace }) };
}

export function bindOf(body: unknown): BindUnboundPiece | null {
  if (typeof body !== "object" || body === null) return null;
  const { unbound_id, repository, replace } = body as Record<string, unknown>;
  if (!named(unbound_id) || !named(repository) || (replace !== undefined && typeof replace !== "boolean")) return null;
  return { unbound_id, repository, ...(replace === undefined ? {} : { replace }) };
}

export type PackageSaveHost = {
  /** The place the person chose, or `undefined` where they closed the dialog. */
  choose: (defaultName: string) => Promise<string | undefined>;
  write: (path: string, text: string) => Promise<void>;
};

/** A file name with no folder in it, ending in the package's extension. */
export function packageNameOf(name: string): string {
  // eslint-disable-next-line no-control-regex -- control characters are what is being taken out
  const clean = basename(name.replace(/[\\/]+/g, "/")).replace(/[\u0000-\u001f\u007f:*?"<>|]+/g, " ").trim();
  const stem = clean.toLowerCase().endsWith(PACKAGE_EXTENSION) ? clean.slice(0, -PACKAGE_EXTENSION.length) : clean;
  return `${stem === "" ? "pieces" : stem}${PACKAGE_EXTENSION}`;
}

export async function savePackageFile(name: unknown, text: unknown, host: PackageSaveHost): Promise<PackageSaved> {
  if (typeof name !== "string" || !named(text) || text.length > MOST_PACKAGE_BYTES) return { saved: false, why: "failed" };
  const chosen = await host.choose(packageNameOf(name));
  if (chosen === undefined) return { saved: false, why: "cancelled" };
  const path = chosen.toLowerCase().endsWith(PACKAGE_EXTENSION) ? chosen : `${chosen}${PACKAGE_EXTENSION}`;
  try {
    await host.write(path, text);
    return { saved: true, name: basename(path) };
  } catch {
    return { saved: false, why: "failed" };
  }
}

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

export function handleSharing({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.commitToRepository, (_event, body: unknown) => {
    const commit = commitOf(body);
    return commit === null ? unsent : (connection()?.sharing.commit(commit) ?? unsent);
  });
  ipc.handle(CHANNELS.sharePackage, (_event, body: unknown) => {
    const share = shareOf(body);
    return share === null ? unsent : (connection()?.sharing.share(share) ?? unsent);
  });
  ipc.handle(CHANNELS.readPackage, (_event, text: unknown) => (named(text) ? (connection()?.sharing.read(text) ?? unsent) : unsent));
  ipc.handle(CHANNELS.addPackage, (_event, body: unknown) => {
    const add = addOf(body);
    return add === null ? unsent : (connection()?.sharing.add(add) ?? unsent);
  });
  ipc.handle(CHANNELS.listUnboundPieces, () => connection()?.sharing.unbound() ?? unsent);
  ipc.handle(CHANNELS.bindUnboundPiece, (_event, body: unknown) => {
    const bind = bindOf(body);
    return bind === null ? unsent : (connection()?.sharing.bind(bind) ?? unsent);
  });
  ipc.handle(CHANNELS.savePackageFile, (event, name: unknown, text: unknown) => {
    const window = BrowserWindow.fromWebContents(event.sender);
    return savePackageFile(name, text, {
      choose: async (defaultName) => {
        const options = { defaultPath: join(app.getPath("downloads"), defaultName), filters: [{ name: "Armada package", extensions: ["toml"] }] };
        const chosen = window === null ? await dialog.showSaveDialog(options) : await dialog.showSaveDialog(window, options);
        return chosen.canceled || chosen.filePath === "" ? undefined : chosen.filePath;
      },
      write: (path, body) => writeFile(path, body, "utf8"),
    });
  });
}
