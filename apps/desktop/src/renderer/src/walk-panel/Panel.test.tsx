// The panel beside the walk window's page, as `main.tsx` mounts it: the Job of the walk on screen drawn as the Cockpit's
// `SessionView`, switching Job as main pushes the next walk's, and nothing where that walk has none (a pull request's).
// `docs/practices/capture-window.md`, *The panel beside the page*.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test } from "vitest";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { NOTHING_YET } from "../../../shared/bridge";
import { SessionsFrom } from "../sessions-draft";
import { Panel } from "./Panel";
import { useWalkSubject } from "./subject";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const session = (id: string, title: string, more: Partial<Session> = {}): Session => ({
  id,
  title,
  attachments: [],
  rows: [{ id: `${id}-1`, at: "2026-10-19T09:00:00Z", kind: "message", from: { kind: "agent" }, text: `${title}, from the agent` }],
  turn: { state: "idle" },
  ...more,
});

const SESSIONS = [
  session("s1", "Retry the lease"),
  session("s2", "Name the slots"),
];
const draft = { models: ["opus"], efforts: ["high"], send: () => {}, tune: () => {}, answer: () => {}, start: () => "x", subscribe: () => () => {}, get: () => SESSIONS } as unknown as SessionsDraft;

/** What the panel's document does with the subject main pushes: `main.tsx`'s `Followed`. */
function Followed() {
  const id = useWalkSubject();
  return <Panel subject={id === undefined ? null : { kind: "session", id }} />;
}

let push: (sessionId: string | undefined) => void = () => {};
let root: Root;
let host: HTMLElement;

beforeEach(() => {
  (window as unknown as { armada: unknown }).armada = {
    subscribeChanges: () => () => {},
    syncState: async () => ({ full: NOTHING_YET, seq: 1 }),
    walkPanel: {
      goInBridge: async () => {},
      openInBridge: async () => {},
      subject: (onChanged: typeof push) => {
        push = onChanged;
        return () => {};
      },
    },
  };
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
});

afterEach(() => {
  act(() => root.unmount());
  host.remove();
});

const frame = (id: string) => host.querySelector(`[role="region"][aria-label="Job ${id}"]`);

async function draw() {
  await act(async () =>
    root.render(
      <SessionsFrom held={draft}>
        <Followed />
      </SessionsFrom>,
    ),
  );
}

test("draws the Job of the walk on screen as the Cockpit's view: its frame, thread and ledger", async () => {
  await draw();
  await act(async () => push("s1"));
  await expect.poll(() => frame("s1")).not.toBeNull();
  expect(host.querySelector('section[aria-label="Job: Retry the lease"]')).not.toBeNull();
  expect(host.querySelector('[role="region"][aria-label="Thread"]')?.textContent).toContain("Retry the lease, from the agent");
  // What the Dashboard's miniature left out: the Job's frame round the thread, and the ledger beside it.
  expect(host.querySelector('aside[aria-label="Attachments"]')).not.toBeNull();
});

test("switches to the next walk's Job as main pushes it", async () => {
  await draw();
  await act(async () => push("s1"));
  await expect.poll(() => frame("s1")).not.toBeNull();
  await act(async () => push("s2"));
  await expect.poll(() => frame("s2")).not.toBeNull();
  expect(frame("s1")).toBeNull();
  expect(host.querySelector('[role="region"][aria-label="Thread"]')?.textContent).toContain("Name the slots, from the agent");
});

test("draws nothing for a walk that has no Job", async () => {
  await draw();
  await act(async () => push("s1"));
  await expect.poll(() => frame("s1")).not.toBeNull();
  await act(async () => push(undefined));
  await expect.poll(() => host.innerHTML).toBe("");
});
