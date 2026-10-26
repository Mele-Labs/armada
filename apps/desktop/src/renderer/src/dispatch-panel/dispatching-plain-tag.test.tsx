// A brief that tags a Session and names no condition only refers to it: no sentence is drawn over the request, the new Session starts at
// once, and the tag goes with the first message. One that names a condition still draws the sentence and waits.

import { forgetAllKept } from "@armada/components";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionsDraft, WaitReading } from "@armada/screens/src/draft/sessions";
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
const retry = { id: "a", title: "Retry loop", rows: [], attachments: [] } as unknown as Session;
const start = vi.fn(async (_tag?: unknown, _root?: string, _request?: string, _tuning?: unknown, _after?: unknown) => "01SESSION");
const send = vi.fn();
const opened = vi.fn();
const SESSIONS = [retry];
/** What the proposer read of the words typed, which the test sets. */
let read: WaitReading;
const draft = {
  get: () => SESSIONS,
  subscribe: () => () => undefined,
  start,
  send,
  models: [],
  efforts: [],
  readsWait: () => read,
  settlesWait: async () => read,
} as unknown as SessionsDraft;

function Outside() {
  held = useDispatching();
  return null;
}

function Window() {
  const dispatching = useDispatch({ state, repositories: [armada], openSession: opened, openAutopilot: () => undefined });
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

test("a tag that names no condition draws no sentence, starts at once and goes with the first message", async () => {
  read = { on: { id: "a", title: "Retry loop" }, when: "ends", base: "main", needs: "", plain: true };
  await expect.poll(() => held).toBeDefined();
  held?.openNew(armada.root);
  const request = page.getByRole("textbox", { name: "Request" });
  await expect.element(request).toBeVisible();
  await userEvent.keyboard("Look at what @Retry loop did to the schema");
  await expect.element(request).toHaveValue("Look at what @Retry loop did to the schema");
  await expect.element(page.getByRole("group", { name: "Start condition" })).not.toBeInTheDocument();
  await userEvent.keyboard("{Enter}");

  await expect.poll(() => start.mock.calls.length).toBe(1);
  expect(start.mock.calls[0]?.[4]).toBeUndefined();
  await expect.poll(() => send.mock.calls.length).toBe(1);
  expect(send).toHaveBeenCalledWith("01SESSION", expect.objectContaining({ text: "Look at what @Retry loop did to the schema", tags: [{ kind: "session", id: "a", title: "Retry loop" }] }));
});

test("a tag that names a condition draws the sentence and waits", async () => {
  read = { on: { id: "a", title: "Retry loop" }, when: "lands", base: "main", needs: "" };
  await expect.poll(() => held).toBeDefined();
  held?.openNew(armada.root);
  await expect.element(page.getByRole("textbox", { name: "Request" })).toBeVisible();
  await userEvent.keyboard("Write the docs once @Retry loop lands");
  await expect.element(page.getByRole("group", { name: "Start condition" })).toBeVisible();
  await userEvent.keyboard("{Enter}");

  await expect.poll(() => start.mock.calls.length).toBe(1);
  expect(start.mock.calls[0]?.[4]).toEqual(expect.objectContaining({ when: "lands" }));
  expect(send).not.toHaveBeenCalled();
});
