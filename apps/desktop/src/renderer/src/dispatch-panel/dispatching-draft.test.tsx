// A brief built for him (an issue, a reported bug, a red main) starts its Job as the sheet's Start would and opens it, where the brief
// waits in draft for his Send. The sheet is never up, and what he had typed in it is not touched. With no repository to start in, the
// sheet opens on the words so he chooses one.

import { forgetAllKept, readKept, writeKept } from "@armada/components";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import "../styles/index.css";
import { connected } from "../mock/moment";
import { issueHit } from "../numbered";
import { SessionsFrom } from "../sessions-draft";
import { DispatchSheet } from "./entries";
import { DispatchingProvider, useDispatch, useDispatching, type Dispatching } from "./dispatching";

let held: Dispatching | undefined;
let mounted: { root: Root; host: HTMLElement } | undefined;
const state = connected([], []);
const armada = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const start = vi.fn(async (_tag?: unknown, _root?: string, _request?: string, _tuning?: unknown) => "01SESSION");
const send = vi.fn();
const openSession = vi.fn();
const NO_SESSIONS: Session[] = [];
const draft = { get: () => NO_SESSIONS, subscribe: () => () => undefined, start, send, models: [], efforts: [], readsWait: () => undefined } as unknown as SessionsDraft;
const dispatchSheet = () => page.getByRole("region", { name: "Dispatch", exact: true });
const { tag } = issueHit({ reference: "#2810", title: "Coupon field accepts an expired code", url: "https://github.com/acme/storefront/issues/2810" });

function Outside() {
  held = useDispatching();
  return null;
}

function Window({ repositories }: { repositories: (typeof armada)[] }) {
  const dispatching = useDispatch({ state, repositories, openSession, openAutopilot: () => undefined });
  return (
    <DispatchingProvider value={dispatching}>
      <Outside />
      <DispatchSheet />
    </DispatchingProvider>
  );
}

function mount(repositories: (typeof armada)[]) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  root.render(
    <SessionsFrom held={draft}>
      <Window repositories={repositories} />
    </SessionsFrom>,
  );
}

beforeEach(() => {
  forgetAllKept();
  start.mockClear();
  send.mockClear();
  openSession.mockClear();
  (window as unknown as { armada: unknown }).armada = { readRequestRepository: async () => ({ manifest_id: null, likely: [] }), searchFiles: async () => [] };
});
afterEach(() => {
  mounted?.root.unmount();
  mounted?.host.remove();
  mounted = undefined;
  held = undefined;
  (window as unknown as { armada?: unknown }).armada = undefined;
});

test("a prepared brief with an issue tag starts one Job, sends it the brief with the tag, and opens it, with the sheet down and his typed words kept", async () => {
  mount([armada]);
  await expect.poll(() => held).toBeDefined();
  writeKept("dispatch:request", "half a thought");

  held?.startDraft(`${tag.title} `, armada.root, [tag]);

  await expect.poll(() => openSession.mock.calls.length).toBe(1);
  expect(openSession).toHaveBeenCalledWith("01SESSION");
  expect(start).toHaveBeenCalledTimes(1);
  expect(start.mock.calls[0]!.slice(1, 3)).toEqual([armada.root, tag.title]);
  expect(send).toHaveBeenCalledTimes(1);
  expect(send).toHaveBeenCalledWith("01SESSION", expect.objectContaining({ text: tag.title, tags: [tag] }));
  expect(held?.sheet).toBe(false);
  expect(dispatchSheet().query()).toBeNull();
  expect(readKept<string>("dispatch:request")).toBe("half a thought");
});

test("a second press on the same brief while its Job is starting drafts no second Job", async () => {
  mount([armada]);
  await expect.poll(() => held).toBeDefined();

  held?.startDraft(`${tag.title} `, armada.root, [tag]);
  held?.startDraft(`${tag.title} `, armada.root, [tag]);

  await expect.poll(() => openSession.mock.calls.length).toBe(1);
  await new Promise((resume) => setTimeout(resume, 200));
  expect(start).toHaveBeenCalledTimes(1);
  expect(send).toHaveBeenCalledTimes(1);
  expect(openSession).toHaveBeenCalledTimes(1);
});

test("a prepared brief whose repository is not found opens the sheet on its words, and starts nothing", async () => {
  mount([]);
  await expect.poll(() => held).toBeDefined();

  held?.startDraft(`${tag.title} `, armada.root, [tag]);

  await expect.element(page.getByRole("textbox", { name: "Request" })).toHaveValue(`${tag.title} `);
  expect(held?.sheet).toBe(true);
  expect(start).not.toHaveBeenCalled();
  expect(openSession).not.toHaveBeenCalled();
});
