// The line for a program the tracker lacks: Install is offered and nothing installs until it is pressed, it reads as work while
// Fleet installs, the search runs again after, and a failed install shows its words with no button left. Browser: the drawing is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";

import type { IssuesSource, Read } from "./model";
import { MissingTool } from "./list";
import { IssuesFrom, createIssuesStore } from "./store";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const missing = { ok: false, said: "searching the issues did not happen: bd is not installed", missing: "bd" } as const;

function mount(install: IssuesSource["install"]) {
  const runs: string[] = [];
  const source = {
    run: async (_manifestId: string, query: string) => (runs.push(query), missing),
    install,
  } as unknown as IssuesSource;
  const store = createIssuesStore(source);
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  act(() =>
    root.render(
      <IssuesFrom store={store}>
        <MissingTool manifestId="storefront" query="status=open" said={missing.said} tool="bd" />
      </IssuesFrom>,
    ),
  );
  return { host, runs };
}

const settle = () => act(() => new Promise<void>((resume) => setTimeout(resume, 0)));
const button = (host: HTMLElement) => host.querySelector("button");

test("the line is kept and Install is offered, and nothing installs until it is pressed", () => {
  let installs = 0;
  const { host } = mount(async () => (installs++, { ok: true, value: undefined }));
  expect(host.querySelector('[role="alert"]')?.textContent).toBe(missing.said);
  expect(button(host)?.textContent).toBe("Install bd");
  expect(installs).toBe(0);
});

test("pressing Install shows the install underway, and the search runs again once it is done", async () => {
  let finish: (read: Read<void>) => void = () => {};
  const { host, runs } = mount(() => new Promise((resume) => (finish = resume)));
  act(() => button(host)?.click());
  expect(button(host)?.textContent).toBe("Installing bd");
  expect(button(host)?.getAttribute("aria-busy")).toBe("true");
  finish({ ok: true, value: undefined });
  await settle();
  expect(runs).toEqual(["status=open"]);
});

test("a failed install shows why and leaves no button to press again", async () => {
  const { host, runs } = mount(async () => ({ ok: false, said: "Homebrew is not available" }));
  act(() => button(host)?.click());
  await settle();
  expect(host.querySelector('[role="alert"]')?.textContent).toBe("Homebrew is not available");
  expect(button(host)).toBeNull();
  expect(runs).toEqual([]);
});
