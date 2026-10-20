// Commit to repository, Share and Copy on a piece of the person's own: a commit names the open Job and says the branch it landed on;
// a refusal is said in Fleet's words, and an id the branch holds offers Replace; Share saves Fleet's package and Copy puts its text on
// the clipboard; a route this Fleet does not serve is the window's toast. Browser, from a fake `window.armada`.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { Outcome } from "@armada/protocol";

import { PieceActs } from "./PieceActs";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const ROOT = "/Users/user/widgets";
const refused = (code: string, message: string): Outcome => ({ ok: false, why: "refused", error: { code, message, run_id: "", fields: {}, chain: [] } });

function fleet(commit: Outcome | undefined = undefined) {
  const armada = {
    commitToRepository: vi.fn(async (body: { overwrite?: boolean }) =>
      commit !== undefined && body.overwrite !== true ? { ok: false, outcome: commit } : { ok: true, value: { repository: ROOT, branch: "mock/widgets", paths: [".armada/flight-controls.toml"], commit: "4f2c9a1" } },
    ),
    sharePackage: vi.fn(async () => ({ ok: true, value: { file_name: "run-tests.armada.toml", text: "version = 1\n" } })),
    savePackageFile: vi.fn(async (name: string) => ({ saved: true, name })),
  };
  (window as unknown as { armada: unknown }).armada = armada;
  return armada;
}

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const flush = () => act(async () => void (await new Promise((done) => setTimeout(done, 0))));

async function mount(job = true) {
  const sessionId = job ? "s1" : undefined;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<PieceActs repository={ROOT} sessionId={sessionId} piece={{ kind: "flight_controls", job: ["run-tests"] }} />));
  roots.push({ root, host });
  return host;
}

const press = async (host: HTMLElement, name: string) => {
  await act(async () => [...host.querySelectorAll("button")].find((one) => one.textContent?.trim() === name)!.click());
  await flush();
};
const said = (host: HTMLElement) => host.querySelector('[role="status"]')?.textContent;

test("Commit names the open Job and says the branch it landed on", async () => {
  const armada = fleet();
  const host = await mount();
  await press(host, "Commit to repository");
  expect(armada.commitToRepository).toHaveBeenCalledWith({ repository: ROOT, session_id: "s1", piece: { kind: "flight_controls", job: ["run-tests"] } });
  expect(said(host)).toBe("Committed to mock/widgets");
});

test("with no Job open the commit is sent without one, and Fleet's refusal is said in its words", async () => {
  const armada = fleet(refused("fleet.commit_no_job", "Open a Job in this repository to commit to its branch"));
  const host = await mount(false);
  await press(host, "Commit to repository");
  expect(armada.commitToRepository).toHaveBeenCalledWith({ repository: ROOT, piece: { kind: "flight_controls", job: ["run-tests"] } });
  expect(said(host)).toBe("Open a Job in this repository to commit to its branch");
});

test("an id the branch holds offers Replace, which commits over it", async () => {
  const armada = fleet(refused("fleet.commit_exists", "The branch already holds run-tests"));
  const host = await mount();
  await press(host, "Commit to repository");
  expect(said(host)).toContain("The branch already holds run-tests");
  await press(host, "Replace");
  expect(armada.commitToRepository).toHaveBeenLastCalledWith(expect.objectContaining({ overwrite: true }));
  expect(said(host)).toBe("Committed to mock/widgets");
});

test("Share saves the package Fleet made, and Copy puts the same text on the clipboard", async () => {
  const armada = fleet();
  const written = vi.fn(async (_text: string) => undefined);
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: written } });
  const host = await mount();
  await press(host, "Share");
  expect(armada.sharePackage).toHaveBeenCalledWith({ pieces: [{ repository: ROOT, piece: { kind: "flight_controls", job: ["run-tests"] } }] });
  expect(armada.savePackageFile).toHaveBeenCalledWith("run-tests.armada.toml", "version = 1\n");
  expect(said(host)).toBe("Saved run-tests.armada.toml");
  await press(host, "Copy");
  expect(written).toHaveBeenCalledWith("version = 1\n");
  expect(said(host)).toBe("Copied");
});

test("a route this Fleet does not serve is raised as the window's toast, and says nothing beside the buttons", async () => {
  const armada = fleet();
  armada.sharePackage.mockResolvedValueOnce({ ok: false, outcome: { ok: false, why: "transport", detail: "404", fault: { method: "POST", path: "/share/package", why: "unanswerable", status: 404 } } } as never);
  const raised = vi.fn();
  window.addEventListener("armada:raise-outcome", raised);
  const host = await mount();
  await press(host, "Share");
  window.removeEventListener("armada:raise-outcome", raised);
  expect(raised).toHaveBeenCalledTimes(1);
  expect(said(host)).toBeUndefined();
});
