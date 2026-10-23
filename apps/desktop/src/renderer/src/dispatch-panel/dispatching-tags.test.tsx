// The request tags what it is about as the Session composer does: `@` offers files under the Jobs, `#` an issue or a pull
// request by number. Both read the repository chosen, so with none chosen `@` offers Jobs alone.

import { forgetAllKept } from "@armada/components";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "../styles/index.css";
import { connected } from "../mock/moment";
import { NumberedFrom, issueHit, pullRequestHit, type NumberedSource } from "../numbered";
import { SessionsFrom } from "../sessions-draft";
import { DispatchSheet } from "./entries";
import { DispatchingProvider, useDispatch, useDispatching, type Dispatching } from "./dispatching";

let held: Dispatching | undefined;
let mounted: { root: Root; host: HTMLElement } | undefined;
const state = connected([], []);
const armada = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const start = vi.fn(async (_tag?: unknown, _root?: string, _request?: string, _tuning?: unknown) => "01SESSION");
const send = vi.fn();
const searchFiles = vi.fn(async (query: string, _manifestId?: string) => ["apps/desktop/src/renderer/src/sessions.tsx", "README.md"].filter((one) => one.toLowerCase().includes(query.toLowerCase())));
const NO_SESSIONS: Session[] = [];
const draft = { get: () => NO_SESSIONS, subscribe: () => () => undefined, start, send, models: [], efforts: [], readsWait: () => undefined } as unknown as SessionsDraft;
const pull = pullRequestHit({ number: 2214, title: "Tag files in the panel" });
const numbered: NumberedSource = { open: (manifestId) => (manifestId === "armada" ? [pull] : []), find: async () => undefined };

function Outside() {
  held = useDispatching();
  return null;
}

function Window({ repositories }: { repositories: (typeof armada)[] }) {
  const dispatching = useDispatch({ state, repositories, openSession: () => undefined, openAutopilot: () => undefined });
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
      <NumberedFrom held={numbered}>
        <Window repositories={repositories} />
      </NumberedFrom>
    </SessionsFrom>,
  );
}

beforeEach(() => {
  forgetAllKept();
  start.mockClear();
  send.mockClear();
  searchFiles.mockClear();
  (window as unknown as { armada: unknown }).armada = { readRequestRepository: async () => ({ manifest_id: null, likely: [] }), searchFiles };
});
afterEach(() => {
  mounted?.root.unmount();
  mounted?.host.remove();
  mounted = undefined;
  held = undefined;
  (window as unknown as { armada?: unknown }).armada = undefined;
});

test("with a repository chosen, @ offers its matching files and choosing one writes @path into the request", async () => {
  mount([armada]);
  await expect.poll(() => held).toBeDefined();
  held?.openNew(armada.root);
  const request = page.getByRole("textbox", { name: "Request" });
  await expect.element(request).toHaveFocus();
  await userEvent.keyboard("look at @sessions");
  // A slow keyboard lets the search fire on `session` first, which already offers sessions.tsx, so wait for the whole word.
  await expect.poll(() => searchFiles.mock.calls.some(([query, id]) => query === "sessions" && id === "armada")).toBe(true);
  await expect.element(page.getByRole("option", { name: /sessions\.tsx/ })).toBeVisible();
  await userEvent.keyboard("{Enter}");
  await expect.element(request).toHaveValue("look at @apps/desktop/src/renderer/src/sessions.tsx ");
});

test("a repository with no Manifest is named by its implicit id, so @ still offers its files", async () => {
  const { manifest: _none, ...bare } = armada;
  const chariot = { ...bare, implicit_id: "chariot-6e8ee791" } as unknown as typeof armada;
  mount([chariot]);
  await expect.poll(() => held).toBeDefined();
  held?.openNew(chariot.root);
  await expect.element(page.getByRole("textbox", { name: "Request" })).toHaveFocus();
  await userEvent.keyboard("look at @sessions");
  await expect.poll(() => searchFiles.mock.calls.some(([query, id]) => query === "sessions" && id === "chariot-6e8ee791")).toBe(true);
  await expect.element(page.getByRole("option", { name: /sessions\.tsx/ })).toBeVisible();
});

test("with no repository chosen, @ offers no files", async () => {
  mount([]);
  await expect.poll(() => held).toBeDefined();
  held?.openNew();
  await expect.element(page.getByRole("textbox", { name: "Request" })).toHaveFocus();
  await userEvent.keyboard("look at @sessions");
  await new Promise((resume) => setTimeout(resume, 400));
  expect(searchFiles).not.toHaveBeenCalled();
  await expect.element(page.getByRole("option", { name: /sessions\.tsx/ })).not.toBeInTheDocument();
});

test("# offers the repository's open pull requests, and choosing one writes it into the request and sends it as a tag", async () => {
  mount([armada]);
  await expect.poll(() => held).toBeDefined();
  held?.openNew(armada.root);
  const request = page.getByRole("textbox", { name: "Request" });
  await expect.element(request).toHaveFocus();
  await userEvent.keyboard("review #22");
  await expect.element(page.getByRole("option", { name: /Tag files in the panel/ })).toBeVisible();
  await userEvent.keyboard("{Enter}");
  await expect.element(request).toHaveValue("review #2214 Tag files in the panel ");
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => send.mock.calls.length).toBe(1);
  expect(send).toHaveBeenCalledWith("01SESSION", expect.objectContaining({ tags: [expect.objectContaining({ kind: "pull_request", id: "2214" })] }));
});

// The Issues tab's Start a Job: the sheet opens on the ticket's words with the ticket held as a tag, and the Job is sent it, which is
// what Fleet claims the ticket from (`issues_named`).
test.each([
  { reference: "#2810", id: "2810", url: "https://github.com/acme/storefront/issues/2810" },
  { reference: "ABC-41", id: "ABC-41", url: "https://acme.atlassian.net/browse/ABC-41" },
])("an issue the Issues tab opens the sheet on is sent with the Job as a tag ($reference)", async ({ reference, id, url }) => {
  mount([armada]);
  await expect.poll(() => held).toBeDefined();
  const { tag } = issueHit({ reference, title: "Coupon field accepts an expired code", url });
  held?.openSheet(`${tag.title} `, armada.root, [tag]);
  await expect.element(page.getByRole("textbox", { name: "Request" })).toHaveValue(`${tag.title} `);
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => send.mock.calls.length).toBe(1);
  expect(send).toHaveBeenCalledWith("01SESSION", expect.objectContaining({ tags: [{ kind: "issue", id, title: `${reference} Coupon field accepts an expired code`, url }] }));
});
