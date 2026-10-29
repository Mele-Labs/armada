// A brief that tags a Session is sent as a Session that waits on it: the real reading is settled before the start, the start carries
// the wait, and nothing is sent as a first message.

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
/** What the proposer read, once it has answered: the pull request lands, from main, and the schema is what is needed. */
const read: WaitReading = { on: { id: "a", title: "Retry loop" }, when: "lands", base: "main", needs: "the schema" };
const settles = vi.fn(async (_words: string) => read);
const SESSIONS = [retry];
const draft = {
  get: () => SESSIONS,
  subscribe: () => () => undefined,
  start,
  send,
  models: [],
  efforts: [],
  readsWait: () => ({ ...read, when: "ends", needs: "" }),
  settlesWait: settles,
} as unknown as SessionsDraft;

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
  settles.mockClear();
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

test("sending a brief that tags a Session waits for the proposer's reading, then starts the Session on the wait and sends no first message", async () => {
  await expect.poll(() => held).toBeDefined();
  held?.openNew(armada.root);
  const request = page.getByRole("textbox", { name: "Request" });
  await expect.element(request).toBeVisible();
  await userEvent.keyboard("Write the docs once @Retry loop lands, so the schema is in");
  await expect.element(request).toHaveValue("Write the docs once @Retry loop lands, so the schema is in");
  await userEvent.keyboard("{Enter}");

  await expect.poll(() => start.mock.calls.length).toBe(1);
  expect(settles).toHaveBeenCalledWith("Write the docs once @Retry loop lands, so the schema is in");
  expect(start).toHaveBeenCalledWith(undefined, armada.root, "Write the docs once @Retry loop lands, so the schema is in", {}, { on: { id: "a", title: "Retry loop" }, when: "lands", base: "main", needs: "the schema" }, undefined, undefined, expect.any(Function));
  await expect.poll(() => opened.mock.calls.length).toBe(1);
  expect(send).not.toHaveBeenCalled();
});
