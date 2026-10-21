// A package pasted or dropped on the window opens a sheet listing every piece and every command it would add, and Add is the one
// ask; an id already held makes the one ask Replace, and a piece left unbound waits for a repository to be chosen. Text pasted into a
// field is the field's. Browser: what is drawn is read by role and name, from a fake `window.armada`.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { PackagePiece, RepositorySummary } from "@armada/protocol";

import { usePasteAndDrop } from "../paste-and-drop";
import { ImportPackage, isPackageText } from "./ImportPackage";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const TEXT = 'version = 1\n\n[[piece]]\nkind = "tile"\nlist = "job"\nremote = "acme/widgets"\n';
const ROOT = "/Users/user/widgets";
const REPOSITORIES = [{ root: ROOT, manifest: { repository: "acme/widgets" } }] as unknown as RepositorySummary[];
const TILE: PackagePiece = { kind: "tile", id: "run-tests", name: "Run tests", list: "job", action: "shell", commands: ["make test"], remote: "acme/widgets", repository: ROOT, exists: false };

function fleet(pieces: PackagePiece[], landed: "added" | "unbound" = "added") {
  let bound = false;
  const armada = {
    readPackage: vi.fn(async () => ({ ok: true, value: { pieces } })),
    addPackage: vi.fn(async () => ({ ok: true, value: { pieces: pieces.map(({ kind, id, name }) => ({ kind, id, name, landed, ...(landed === "unbound" ? { unbound_id: "u1" } : { repository: ROOT }) })) } })),
    listUnboundPieces: vi.fn(async () => ({ ok: true, value: { pieces: landed === "unbound" && !bound ? [{ unbound_id: "u1", kind: "tile", id: "run-tests", name: "Run tests", commands: ["make test"], remote: "elsewhere/widgets" }] : [] } })),
    bindUnboundPiece: vi.fn(async () => ((bound = true), { ok: true, value: { kind: "tile", id: "run-tests", name: "Run tests", repository: ROOT, landed: "added" } })),
  };
  (window as unknown as { armada: unknown }).armada = armada;
  return armada;
}

/** The shell's one listener, which the window's import is claimed from. */
function Shell() {
  usePasteAndDrop();
  return <ImportPackage repositories={REPOSITORIES} />;
}

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const flush = () => act(async () => void (await new Promise((done) => setTimeout(done, 0))));

async function mount() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Shell />));
  roots.push({ root, host });
  await flush();
  return host;
}

async function paste(text: string, on: EventTarget = document.body) {
  const data = new DataTransfer();
  data.setData("text/plain", text);
  await act(async () => void on.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true })));
  await flush();
}

const sheet = () => document.querySelector<HTMLElement>('[role="dialog"][aria-label="Import"], [role="dialog"]');
const marked = (name: string) => sheet()?.querySelector(`[role="img"][aria-label="${name}"]`) ?? null;
const button = (name: string) => [...document.querySelectorAll("button")].find((one) => one.textContent?.trim() === name);

test("package text is its version line and a piece, and nothing else is", () => {
  expect(isPackageText(TEXT)).toBe(true);
  expect(isPackageText("version = 1\n")).toBe(false);
  expect(isPackageText("[[piece]]\nkind = 'tile'")).toBe(false);
});

test("pasted, it lists every piece with the commands it runs, writing nothing until Add", async () => {
  const armada = fleet([TILE]);
  await mount();
  await paste(TEXT);
  expect(armada.readPackage).toHaveBeenCalledWith(TEXT);
  expect(sheet()?.querySelector('[aria-label="Tile Run tests"]')?.textContent).toContain("Run tests");
  expect(sheet()?.textContent).toContain("make test");
  expect(sheet()?.textContent).toContain("acme/widgets");
  expect(armada.addPackage).not.toHaveBeenCalled();
  await act(async () => button("Add")!.click());
  await flush();
  expect(armada.addPackage).toHaveBeenCalledWith({ text: TEXT });
  expect(marked("Added")).not.toBeNull();
  expect(sheet()?.textContent).toContain("make test");
});

test("an id already held makes the one ask Replace", async () => {
  const armada = fleet([{ ...TILE, exists: true }]);
  await mount();
  await paste(TEXT);
  expect(marked("Replaces yours")).not.toBeNull();
  expect(button("Add")).toBeUndefined();
  await act(async () => button("Replace")!.click());
  await flush();
  expect(armada.addPackage).toHaveBeenCalledWith({ text: TEXT, replace: true });
});

test("a piece left unbound waits for a repository, and Choose writes it there", async () => {
  const armada = fleet([{ ...TILE, remote: "elsewhere/widgets", repository: undefined } as unknown as PackagePiece], "unbound");
  await mount();
  await paste(TEXT);
  await act(async () => button("Add")!.click());
  await flush();
  expect(armada.listUnboundPieces).toHaveBeenCalled();
  expect(marked("Waiting for a repository")).not.toBeNull();
  await act(async () => button("Choose")!.click());
  await flush();
  expect(armada.bindUnboundPiece).toHaveBeenCalledWith({ unbound_id: "u1", repository: ROOT });
  expect(marked("Added")).not.toBeNull();
  expect(marked("Waiting for a repository")).toBeNull();
});

test("text pasted into a field is the field's", async () => {
  const armada = fleet([TILE]);
  await mount();
  const field = document.createElement("textarea");
  document.body.append(field);
  await paste(TEXT, field);
  field.remove();
  expect(armada.readPackage).not.toHaveBeenCalled();
});
