// The window's Sessions feed the Judge: where they serve one, its acts and each Job's `judgeReadings` reach the components; where they serve none, nothing is drawn. Browser: a
// rendering is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { JudgeReading } from "@armada/protocol";
import type { Session, SessionJudgeActs, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { SessionsFrom } from "../sessions-draft";
import { JudgeAct } from "./JudgeAct";
import "../styles/index.css";

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

const ARTIFACT = {
  kind: "pull_request" as const,
  id: "2418",
  title: "#2418",
};
const reading = (more: Partial<JudgeReading> = {}): JudgeReading => ({
  id: "r1",
  artifact: ARTIFACT,
  prompt: "",
  state: "done",
  asked_at: "2026-10-31T14:00:00Z",
  read: false,
  notes: [
    {
      id: "n1",
      claim: "No ceiling",
    },
  ],
  ...more,
});
const job = (readings?: readonly JudgeReading[]): Session =>
  ({
    id: "job-1",
    rows: [],
    attachments: [],
    turn: {
      state: "idle",
    },
    ...(readings === undefined
      ? {}
      : {
          judgeReadings: readings,
        }),
  }) as unknown as Session;

const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

/** A draft whose Sessions can change: `set` replaces them and tells whoever listens. */
function draftOf(first: Session[], judge?: SessionJudgeActs) {
  let sessions = first;
  const listeners = new Set<() => void>();
  const draft = {
    get: () => sessions,
    subscribe: (on: () => void) => (listeners.add(on), () => void listeners.delete(on)),
    ...(judge === undefined
      ? {}
      : {
          judge,
        }),
  } as unknown as SessionsDraft;
  return {
    draft,
    set: (next: Session[]) => act(() => ((sessions = next), listeners.forEach((one) => one()))),
  };
}

function mount(draft: SessionsDraft) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() =>
    root.render(
      <SessionsFrom held={draft}>
        <JudgeAct job="job-1" artifact={ARTIFACT} label="Judge #2418" />
      </SessionsFrom>,
    ),
  );
  roots.push({
    root,
    host,
  });
  return host;
}

const acts = (): SessionJudgeActs => ({
  ask: vi.fn(),
  send: vi.fn(),
  dismiss: vi.fn(),
  markRead: vi.fn(),
});
const press = (host: HTMLElement) => host.querySelector("button[aria-haspopup='dialog']");
const mark = (host: HTMLElement, name: string) =>
  host.querySelector(`[role="img"][aria-label="${name}"]`);

test("Sessions that serve no Judge draw no Judge act", () => {
  const host = mount(
    draftOf([
      job([
        reading(),
      ]),
    ]).draft,
  );
  expect(press(host)).toBeNull();
});

test("a Job's readings reach the act that reads them", () => {
  const host = mount(
    draftOf(
      [
        job([
          reading(),
        ]),
      ],
      acts(),
    ).draft,
  );
  expect(press(host)).not.toBeNull();
  expect(mark(host, "Notes to read")).not.toBeNull();
});

test("a reading that finishes changes the act when the Job's record does", () => {
  const { draft, set } = draftOf(
    [
      job([
        reading({
          state: "reading",
          notes: [],
        }),
      ]),
    ],
    acts(),
  );
  const host = mount(draft);
  expect(mark(host, "Judge reading")).not.toBeNull();
  set([
    job([
      reading({
        notes: [],
      }),
    ]),
  ]);
  expect(mark(host, "Judge reading")).toBeNull();
  expect(mark(host, "Nothing missed")).not.toBeNull();
});

test("a press in the box asks the Sessions' act with the Job", () => {
  const judge = acts();
  const host = mount(
    draftOf(
      [
        job(),
      ],
      judge,
    ).draft,
  );
  act(() => (press(host) as HTMLElement).click());
  act(() =>
    [
      ...document.body.querySelectorAll<HTMLElement>('[role="dialog"] button'),
    ]
      .find((one) => one.textContent === "Judge")!
      .click(),
  );
  expect(judge.ask).toHaveBeenCalledWith("job-1", ARTIFACT, "");
});
