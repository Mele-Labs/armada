// A Session dispatched from the sheet carries the words as `request`, so Fleet can read them once, and still sends
// them as the first message.

import { forgetAllKept } from "@armada/components";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "../styles/index.css";
import { connected } from "../mock/moment";
import { SessionsFrom } from "../sessions-draft";
import { DispatchSheet } from "./entries";
import { DispatchingProvider, useDispatch, useDispatching, type Dispatching } from "./dispatching";

let held: Dispatching | undefined;
let mounted: { root: Root; host: HTMLElement } | undefined;
const state = connected([], []);
const armada = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const start = vi.fn(async (_tag?: unknown, _root?: string, _request?: string, _tuning?: unknown) => "01SESSION");
const send = vi.fn();
const opened = vi.fn();
// One list, since the sheet reads the Sessions through `useSyncExternalStore` and a new array each read never settles.
const NO_SESSIONS: Session[] = [];
const draft = { get: () => NO_SESSIONS, subscribe: () => () => undefined, start, send, models: [], efforts: [] } as unknown as SessionsDraft;

function Outside() {
  held = useDispatching();
  return null;
}

function Window() {
  const dispatching = useDispatch({
    state,
    repositories: [armada],
    openSession: opened,
    openAutopilot: () => undefined,
  });
  return (
    <DispatchingProvider value={dispatching}>
      <Outside />
      <DispatchSheet />
    </DispatchingProvider>
  );
}

beforeEach(() => {
  forgetAllKept();
  start.mockClear();
  send.mockClear();
  opened.mockClear();
  (window as unknown as { armada: unknown }).armada = { readRequestRepository: async () => ({ manifest_id: null, likely: [] }), searchFiles: async () => [] };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  root.render(
    <SessionsFrom held={draft}>
      <Window />
    </SessionsFrom>,
  );
});
afterEach(() => {
  mounted?.root.unmount();
  mounted?.host.remove();
  mounted = undefined;
  held = undefined;
  (window as unknown as { armada?: unknown }).armada = undefined;
});

test("starting a Session from the sheet passes the typed words as the request and still sends them as the first message", async () => {
  await expect.poll(() => held).toBeDefined();
  held?.openNew(armada.root);
  const request = page.getByRole("textbox", { name: "Request" });
  await expect.element(request).toBeVisible();
  await expect.element(request).toHaveFocus();
  await expect.element(page.getByRole("button", { name: "Start" })).toBeVisible();
  await userEvent.keyboard("Fix the login page");
  await expect.element(request).toHaveValue("Fix the login page");
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => start.mock.calls.length).toBe(1);
  expect(start).toHaveBeenCalledWith(undefined, armada.root, "Fix the login page", {}, undefined, true);
  await expect.poll(() => send.mock.calls.length).toBe(1);
  expect(send).toHaveBeenCalledWith("01SESSION", expect.objectContaining({ text: "Fix the login page" }));
  expect(opened).toHaveBeenCalledWith("01SESSION");
});
