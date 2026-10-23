// A Job's terminal window as its document mounts it: titled for the Job, the shell's screen focused with what it printed before
// the window came to it, Esc left to the shell, and a head whose moves send the shell back into Bridge.

import "../styles/terminal-window.css";

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { SessionRecord, SessionRunMessage } from "@armada/protocol";

import { NOTHING_YET } from "../../../shared/bridge";
import { TerminalWindow } from "./TerminalWindow";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const SESSION = "s60";
const PRINTED = "~/armada $ ls\r\nCONTRIBUTING.md\r\n~/armada $ ";

function fakeApi() {
  let ran: (runId: string, message: SessionRunMessage) => void = () => undefined;
  const full = { ...NOTHING_YET, sessions: { state: "read", sessions: [{ id: SESSION, title: "Deploy the store" } as SessionRecord] } };
  return {
    print: (text: string) => ran("r1", { kind: "output", text }),
    api: {
      subscribeChanges: () => () => undefined,
      syncState: async () => ({ full, seq: 1 }),
      openTerminal: vi.fn(async () => ({ ok: true as const, value: { run_id: "r1" } })),
      readTerminalSoFar: vi.fn(async () => PRINTED),
      writeSessionRun: vi.fn(async () => ({ ok: true as const, value: { run_id: "r1" } })),
      resizeSessionRun: vi.fn(async () => ({ ok: true as const, value: { run_id: "r1" } })),
      stopSessionRun: vi.fn(async () => ({ ok: true as const, value: { run_id: "r1" } })),
      onSessionRunMessage: (on: typeof ran) => ((ran = on), () => undefined),
      onSessionTerminal: () => () => undefined,
      moveTerminalBack: vi.fn(async () => undefined),
      saveSettings: vi.fn(async () => ({ ok: true })),
    },
  };
}

let root: Root;
let host: HTMLElement;
let fake: ReturnType<typeof fakeApi>;

beforeEach(() => {
  fake = fakeApi();
  (window as unknown as { armada: unknown }).armada = fake.api;
  host = document.createElement("div");
  host.style.cssText = "display: flex; flex-direction: column; width: 900px; height: 520px";
  document.body.append(host);
  root = createRoot(host);
});

afterEach(() => {
  act(() => root.unmount());
  host.remove();
});

const pane = () => page.getByRole("group", { name: "Terminal", exact: true });

async function opened() {
  await act(async () => root.render(<TerminalWindow api={fake.api as unknown as Parameters<typeof TerminalWindow>[0]["api"]} sessionId={SESSION} />));
  await expect.element(pane().getByText("CONTRIBUTING.md")).toBeVisible();
}

test("is titled Terminal with the Job's title, and draws what the shell printed before it came, focused to type into", async () => {
  await opened();
  await expect.poll(() => document.title).toBe("Terminal: Deploy the store");
  expect(fake.api.openTerminal).toHaveBeenCalledTimes(1);
  expect(fake.api.readTerminalSoFar).toHaveBeenCalledWith("r1");
  await expect.poll(() => document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
  fake.print("pwd\r\n");
  await expect.element(pane().getByText("pwd")).toBeVisible();
});

test("leaves Esc to the shell: it is typed, and nothing in the window closes", async () => {
  await opened();
  await expect.poll(() => document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => fake.api.writeSessionRun.mock.calls.map((call) => (call as unknown[])[1])).toContain("\u001b");
  await expect.element(pane()).toBeVisible();
  expect(fake.api.moveTerminalBack).not.toHaveBeenCalled();
});

test("its head moves the shell back into Bridge, docked or in the side panel, and saves where it opens; the window's own close stands for Hide and Close", async () => {
  await opened();
  expect(pane().getByRole("button", { name: "Hide" }).query()).toBeNull();
  expect(pane().getByRole("button", { name: "Open in a window" }).query()).toBeNull();
  await pane().getByRole("button", { name: "Dock" }).click();
  expect(fake.api.moveTerminalBack).toHaveBeenLastCalledWith(SESSION, "docked");
  expect(fake.api.saveSettings).toHaveBeenLastCalledWith({ "terminal.openJobTerminalIn": "docked" });
  await pane().getByRole("button", { name: "Move to the side panel" }).click();
  expect(fake.api.moveTerminalBack).toHaveBeenLastCalledWith(SESSION, "side panel");
  expect(fake.api.saveSettings).toHaveBeenLastCalledWith({ "terminal.openJobTerminalIn": "side panel" });
});
