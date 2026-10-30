// What a Job's Workflow offers to open, read off what Fleet serves: the head of a lane for a Job it started opens that Job's own Workflow where Fleet serves that Job's lifecycles, and has no
// button where it does not. The Jobs are the real store's, fed `get_job_lifecycles` answers by hand, so the Open on a head and the glyph of a handed ticket come from one read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import type { JobLifecycles, SessionRecord } from "@armada/protocol";

import "../styles/index.css";
import type { BridgeApi } from "../../../shared/api";
import { NOTHING_YET } from "../../../shared/bridge";
import type { BridgeState } from "../../../shared/bridge";
import type { StateChange } from "../../../shared/state-change";
import { SessionsFrom } from "../sessions-draft";
import { WiredStore } from "../sessions-wired";
import { LanesBody } from "./lanes";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const AT = "2026-10-30T12:00:00Z";

const record = (id: string, title: string): SessionRecord => ({
  id,
  title,
  harness: "a_harness",
  origin: "bridge",
  manifest_id: "armada",
  cwd: "/repo",
  state: "live",
  started_at: AT,
  last_seen_at: AT,
  usage: {},
  attachments: [],
  hosted: { turn: { state: "idle" }, mode: "auto", running: false },
});

const TICKET: JobLifecycles["lifecycles"][number] = {
  id: "ticket",
  name: "Ticket",
  scope: { kind: "everywhere" },
  states: [
    { id: "todo", label: "To do", read: { source: "tracker", words: ["To Do"] } },
    { id: "in_review", label: "In review", read: { source: "tracker", words: ["In Review"] } },
  ],
  transitions: [{ from: "todo", to: "in_review" }],
  ends: [],
};
const JOB: JobLifecycles["lifecycles"][number] = { id: "job", name: "Job", scope: { kind: "everywhere" }, states: [{ id: "started", label: "Started", read: { source: "fleet", fact: "started" } }], transitions: [], ends: [] };

/** `parent` started three Jobs: `child` for a ticket, `quiet` whose lifecycles Fleet does not serve, and `gone`, which is no Job the window holds. */
const parent: JobLifecycles = {
  session_id: "parent",
  lifecycles: [JOB, TICKET],
  instances: [
    { lifecycle: "job", key: "child", label: "Fix cart rounding", standing: { kind: "in", state: "started" }, links: [{ lifecycle: "ticket", key: "ARM-52" }] },
    { lifecycle: "job", key: "quiet", label: "Tidy the panel", standing: { kind: "in", state: "started" }, links: [] },
    { lifecycle: "job", key: "gone", label: "Old attempt", standing: { kind: "in", state: "started" }, links: [] },
    { lifecycle: "ticket", key: "ARM-52", label: "ARM-52", standing: { kind: "in", state: "todo" }, links: [], ended_at: AT },
  ],
  moves: [],
};
const child = (state: string): JobLifecycles => ({
  session_id: "child",
  lifecycles: [TICKET],
  instances: [{ lifecycle: "ticket", key: "ARM-52", label: "ARM-52", standing: { kind: "in", state }, links: [] }],
  moves: [],
});

/** The window's store, with Fleet answering `get_job_lifecycles` for `parent` and `child` and refusing it for `quiet`, and `job_lifecycles.changed` sent by hand. */
async function mounted() {
  let send: (change: StateChange<BridgeState>) => void = () => undefined;
  let changed: (held: JobLifecycles) => void = () => undefined;
  const api = {
    syncState: () => Promise.resolve({ full: NOTHING_YET, seq: 0 }),
    subscribeChanges: (on: typeof send) => ((send = on), () => undefined),
    getJobLifecycles: vi.fn(async (id: string) => (id === "parent" ? { ok: true as const, value: parent } : id === "child" ? { ok: true as const, value: child("in_review") } : { ok: false as const, outcome: { ok: false as const, why: "not_connected" as const } })),
    listLifecycleRules: async () => ({ ok: true as const, value: { manifest_id: "armada", rules: [], left_out: [] } }),
    onJobLifecyclesChanged: (on: typeof changed) => ((changed = on), () => undefined),
  } as unknown as BridgeApi;
  const store = new WiredStore(api);
  store.attach();
  send({ full: structuredClone({ ...NOTHING_YET, sessions: { state: "read", sessions: [record("parent", "Go through app feedback"), record("child", "Fix cart rounding"), record("quiet", "Tidy the panel")] } }), seq: 0 });
  await vi.waitFor(() => expect(store.draft.get().find((one) => one.id === "child")?.lanes).toBeDefined());
  const host = document.createElement("div");
  host.style.cssText = "width:900px;height:600px";
  document.body.append(host);
  const root = createRoot(host);
  act(() =>
    root.render(
      <SessionsFrom held={store.draft}>
        <LanesBody sessionId="parent" />
      </SessionsFrom>,
    ),
  );
  roots.push({ root, host });
  return { changed: (held: JobLifecycles) => act(() => changed(held)) };
}

const opening = (title: string) => page.getByRole("button", { name: `Open Workflow: ${title}`, exact: true });

test("the head of a lane for a Job whose lifecycles Fleet serves opens that Job's Workflow", async () => {
  await mounted();
  await expect.element(opening("Fix cart rounding")).toBeVisible();
});

test("the head of a lane for a Job whose lifecycles Fleet does not serve has no button, nor one for a Job the window does not hold", async () => {
  await mounted();
  await expect.element(page.getByText("Tidy the panel", { exact: true })).toBeVisible();
  await expect.element(page.getByText("Old attempt", { exact: true })).toBeVisible();
  await expect.element(opening("Tidy the panel")).not.toBeInTheDocument();
  await expect.element(opening("Old attempt")).not.toBeInTheDocument();
});

test("a handed ticket has no lane of its own, and its glyph follows what the started Job's Fleet says", async () => {
  const { changed } = await mounted();
  await expect.element(page.getByRole("button", { name: "Fix cart rounding: Started, ticket ARM-52, In review", exact: true })).toBeVisible();
  await expect.element(page.getByText("ARM-52", { exact: true })).toBeVisible();
  changed(child("todo"));
  await expect.element(page.getByRole("button", { name: "Fix cart rounding: Started, ticket ARM-52, To do", exact: true })).toBeVisible();
  // The ticket is on the card, and no lane is titled for it.
  expect(Array.from(document.querySelectorAll(".armada-lane__title"), (one) => one.textContent)).toEqual(["Fix cart rounding", "Tidy the panel", "Old attempt"]);
});
