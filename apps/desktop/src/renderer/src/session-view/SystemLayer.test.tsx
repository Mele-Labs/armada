// The System layer on the mock Fleet's answers: a stopped process stays in its list as stopped though the next reading no longer has it, and a log opens in the file viewer.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { SessionSystem } from "@armada/protocol";

import { sessionSystem, seedSystem } from "../mock/slices/session-system";
import { openInPanel } from "../shared-panel/store";
import { SystemSections } from "./SystemLayer";

vi.mock("../shared-panel/store", () => ({ openInPanel: vi.fn(() => true) }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
beforeEach(() => {
  (window as unknown as { armada: unknown }).armada = sessionSystem.api(undefined as never, undefined as never);
});
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  vi.clearAllMocks();
});

function seed(id: string): SessionSystem {
  const system: SessionSystem = {
    session_id: id,
    processes: [
      { pid: 4101, command: "vite --port 5501", port: 5501, state: "running" },
      { pid: 4102, command: "vitest watch", state: "running" },
    ],
    worktrees: [],
    logs: [{ name: "session.log", path: `/keepers/${id}.log`, bytes: 2048 }],
  };
  seedSystem(system);
  return system;
}

async function mount(id: string) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<SystemSections sessionId={id} worktrees={null} />));
  roots.push({ root, host });
  return host;
}

const row = (host: HTMLElement, command: string) => host.querySelector<HTMLElement>(`li.sv-process[aria-label="${command}"]`);

test("a process that is stopped stays in the list as stopped though the next reading no longer has it", async () => {
  seed("stop-keeps");
  const host = await mount("stop-keeps");
  expect(row(host, "vite --port 5501")?.dataset.state).toBe("running");

  await act(async () => host.querySelector<HTMLElement>('[aria-label="Stop vite --port 5501"]')!.click());

  expect(row(host, "vite --port 5501")?.dataset.state).toBe("stopped");
  expect(host.querySelector('[aria-label="Stop vite --port 5501"]'), "a stopped row offers no Stop").toBeNull();
  expect(row(host, "vitest watch")?.dataset.state, "the other one still runs").toBe("running");
});

test("a log row opens its file in the panel by its path", async () => {
  seed("log-opens");
  const host = await mount("log-opens");

  act(() => host.querySelector<HTMLElement>('[aria-label="Open session.log"]')!.click());

  expect(openInPanel).toHaveBeenCalledWith({ kind: "file", path: "/keepers/log-opens.log", from: "session", sessionId: "log-opens" });
});
