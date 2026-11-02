// The line a finished reading leaves in the thread, and where it stands among the rows. Browser: a rendering and a press are what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { JudgeReading } from "@armada/protocol";
import { SessionThread, type SessionThreadRow } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { openAtNotes } from "./artifacts";
import { provideJudge, putReadings } from "./store";
import { useJudgedRows } from "./thread";
import "../styles/index.css";

vi.mock("./artifacts", () => ({
  openAtNotes: vi.fn(),
  artifactOfRow: () => undefined,
}));

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

const JOB = "job-1";
const ARTIFACT = {
  kind: "pull_request" as const,
  id: "2418",
  title: "#2418",
};
const SESSION = {
  id: JOB,
  rows: [],
  attachments: [],
} as unknown as Session;

const say = (id: string, at: string, text: string): SessionThreadRow => ({
  id,
  at,
  kind: "message",
  from: "agent",
  text,
});
const reading = (more: Partial<JudgeReading>): JudgeReading => ({
  id: "r1",
  artifact: ARTIFACT,
  prompt: "",
  state: "done",
  asked_at: "2026-10-31T14:00:00Z",
  finished_at: "2026-10-31T14:05:00Z",
  read: false,
  notes: [],
  ...more,
});

function Thread({ rows }: { rows: readonly SessionThreadRow[] }) {
  return (
    <SessionThread
      rows={useJudgedRows(SESSION, rows)}
      onAnswer={() => undefined}
      onOpenSession={() => undefined}
    />
  );
}

const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];
beforeEach(() =>
  provideJudge({
    ask: vi.fn(),
    send: vi.fn(),
    dismiss: vi.fn(),
    markRead: vi.fn(),
  }),
);
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  provideJudge(undefined);
  vi.mocked(openAtNotes).mockClear();
});

function mount(rows: readonly SessionThreadRow[]) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Thread rows={rows} />));
  roots.push({
    root,
    host,
  });
  return host;
}

const line = (host: HTMLElement) =>
  host.querySelector<HTMLElement>('button[aria-label="Judged #2418"]');
const ROWS = [
  say("a", "2026-10-31T14:01:00Z", "First."),
  say("b", "2026-10-31T14:10:00Z", "Later."),
];

test("a finished reading leaves a Judged line in the thread", () => {
  act(() =>
    putReadings(JOB, [
      reading({}),
    ]),
  );
  const host = mount(ROWS);
  expect(line(host)).not.toBeNull();
  expect(line(host)!.textContent).toContain("Judged");
});

test("a reading still going, or one that failed, leaves none", () => {
  act(() =>
    putReadings(JOB, [
      reading({
        state: "reading",
      }),
      reading({
        id: "r2",
        artifact: {
          kind: "commit",
          id: "a41c9e2",
          title: "a41c9e2",
        },
        state: "failed",
        failed: "The read stopped",
      }),
    ]),
  );
  const host = mount(ROWS);
  expect(host.querySelector('button[aria-label^="Judged"]')).toBeNull();
});

test("the line stands after the last row that happened by the time the reading finished", () => {
  act(() =>
    putReadings(JOB, [
      reading({}),
    ]),
  );
  const host = mount(ROWS);
  const order = [
    ...host.querySelectorAll("li"),
  ].map((one) => one.textContent ?? "");
  const at = order.findIndex((text) => text.includes("Judged"));
  expect(order.findIndex((text) => text.includes("First."))).toBeLessThan(at);
  expect(order.findIndex((text) => text.includes("Later."))).toBeGreaterThan(at);
});

test("rows with no time of their own leave the line at the end", () => {
  act(() =>
    putReadings(JOB, [
      reading({}),
    ]),
  );
  const host = mount([
    say("a", "14:01:00", "First."),
    say("b", "14:10:00", "Later."),
  ]);
  const order = [
    ...host.querySelectorAll("li"),
  ].map((one) => one.textContent ?? "");
  expect(order.findIndex((text) => text.includes("Judged"))).toBeGreaterThan(
    order.findIndex((text) => text.includes("Later.")),
  );
});

test("the line holds no button inside its press", () => {
  act(() =>
    putReadings(JOB, [
      reading({}),
    ]),
  );
  const host = mount(ROWS);
  expect(line(host)!.querySelector("button")).toBeNull();
});

test("a press on the line opens the artifact at its notes", () => {
  act(() =>
    putReadings(JOB, [
      reading({
        notes: [
          {
            id: "n1",
            claim: "No ceiling",
          },
        ],
      }),
    ]),
  );
  const host = mount(ROWS);
  act(() => line(host)!.click());
  expect(openAtNotes).toHaveBeenCalledWith(SESSION, ARTIFACT);
});
