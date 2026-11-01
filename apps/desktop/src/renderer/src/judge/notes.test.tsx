// A judge's notes inside the artifact: under the diff line of a pull request that a note names, with Send to Job and Dismiss on each. Browser: a rendering and a press are what
// are read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { JudgeReading } from "@armada/protocol";
import { UnifiedDiff, type DiffFile } from "@armada/components";

import { newLineAt, useDiffUnder } from "./notes";
import { provideJudge, putReadings, type JudgeApi, type JudgeArtifact } from "./store";
import "../styles/index.css";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const JOB = "job-1";
const ARTIFACT: JudgeArtifact = { kind: "pull_request", id: "2418", title: "#2418" };
const TOKENS = "crates/api/src/auth/tokens.rs";

/** Line 41 of the new file is the one added; the hunk starts at 38. */
const FILE: DiffFile = {
  path: TOKENS,
  lines: [
    { kind: "hunk", text: "@@ -38,3 +38,4 @@ impl SessionToken {" },
    { kind: "context", text: "     pub fn issue(config: &Config) -> Self {" },
    { kind: "removed", text: "-            issued: Utc::now()," },
    { kind: "context", text: "         Self {" },
    { kind: "context", text: "             id: Uuid::new_v4()," },
    { kind: "added", text: "+            ttl: Duration::from_secs(config.ttl)," },
    { kind: "context", text: "         }" },
  ],
};

const reading = (more: Partial<JudgeReading> = {}): JudgeReading => ({
  id: "r1",
  artifact: ARTIFACT,
  prompt: "",
  state: "done",
  asked_at: "2026-10-31T14:00:00Z",
  finished_at: "2026-10-31T14:02:00Z",
  read: false,
  notes: [{ id: "n2", claim: "A token never expires when its lifetime is zero.", where: `${TOKENS}:41`, about: `${TOKENS}:41` }],
  ...more,
});

function Diff() {
  const under = useDiffUnder(JOB, ARTIFACT);
  return <UnifiedDiff files={[FILE]} {...(under === undefined ? {} : { under })} />;
}

const roots: { root: Root; host: HTMLElement }[] = [];
let judge: JudgeApi;
beforeEach(() => {
  judge = { ask: vi.fn(), send: vi.fn(), dismiss: vi.fn(), markRead: vi.fn() };
  provideJudge(judge);
});
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  provideJudge(undefined);
});

function mount() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Diff />));
  roots.push({ root, host });
  return host;
}

const note = (host: HTMLElement) => host.querySelector<HTMLElement>('[role="group"][aria-label^="Finding: A token never expires"]');
const after = (a: Node, b: Node) => Boolean(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING);
const text = (host: HTMLElement, words: string) => [...host.querySelectorAll<HTMLElement>("*")].filter((one) => one.children.length === 0 && one.textContent?.includes(words)).at(-1)!;

test("the line a diff line has in the new file is counted from the hunk header", () => {
  expect(newLineAt(FILE, 1)).toBe(38);
  expect(newLineAt(FILE, 2)).toBeUndefined();
  expect(newLineAt(FILE, 3)).toBe(39);
  expect(newLineAt(FILE, 5)).toBe(41);
  expect(newLineAt(FILE, 6)).toBe(42);
});

test("a note stands under the diff line it names", () => {
  act(() => putReadings(JOB, [reading()]));
  const host = mount();
  expect(note(host)).not.toBeNull();
  expect(after(text(host, "ttl: Duration"), note(host)!)).toBe(true);
  expect(after(note(host)!, text(host, "         }"))).toBe(true);
  expect(note(host)!.textContent).toContain("You should know");
});

test("a diff with no note on a line draws as it did", () => {
  act(() => putReadings(JOB, [reading({ notes: [{ id: "n1", claim: "Whole pull request", where: "#2418" }] })]));
  const host = mount();
  expect(host.querySelector('[role="group"][aria-label^="Finding:"]')).toBeNull();
});

test("Send to Job gives the note to the api", () => {
  act(() => putReadings(JOB, [reading()]));
  const host = mount();
  act(() => [...note(host)!.querySelectorAll("button")].find((one) => one.textContent?.includes("Send to Job"))!.click());
  expect(judge.send).toHaveBeenCalledWith("r1", "n2");
  expect(judge.dismiss).not.toHaveBeenCalled();
});

test("Dismiss gives the note to the api", () => {
  act(() => putReadings(JOB, [reading()]));
  const host = mount();
  act(() => [...note(host)!.querySelectorAll("button")].find((one) => one.textContent?.includes("Dismiss"))!.click());
  expect(judge.dismiss).toHaveBeenCalledWith("r1", "n2");
  expect(judge.send).not.toHaveBeenCalled();
});

test("a note drawn is a note read", () => {
  act(() => putReadings(JOB, [reading()]));
  mount();
  expect(judge.markRead).toHaveBeenCalledWith("r1");
});

test("a note settled folds to one line with no acts", () => {
  act(() => putReadings(JOB, [reading({ notes: [{ id: "n2", claim: "A token never expires when its lifetime is zero.", about: `${TOKENS}:41`, settled: { how: "sent", at: "2026-10-31T14:03:00Z" } }] })]));
  const host = mount();
  expect(host.querySelector('[aria-label^="Finding sent:"]')).not.toBeNull();
  expect(note(host)).toBeNull();
});
