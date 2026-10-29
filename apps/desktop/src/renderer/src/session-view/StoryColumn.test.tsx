// What leads Now in the Job's story: the line worked out from the Job's tool calls, ahead of any task the agent has marked, and the agent by name when nothing is
// being done. Browser: the drawing is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { Session, SessionWork } from "@armada/screens/src/draft/sessions";

import type { PlanEditing } from "./PlanEdit";
import { StoryColumn } from "./StoryColumn";
import { resetFolds } from "./story";

// The mod list is Fleet's, read through `window.armada`; no Job here made a mod.
vi.mock("../mod-list", () => ({ useModList: () => [] }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const ID = "story-column-s1";
const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  resetFolds(ID);
  localStorage.clear();
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const plan = { scope: [], approved: true, groups: [{ id: "g1", title: "Retry the lease", agent: "rust-engineer", paths: [], tasks: [{ id: "t1", text: "Name the failure", state: "working" as const }, { id: "t2", text: "Retry once", state: "open" as const }] }] };
const work = { plan } as unknown as SessionWork;
const ed: PlanEditing = { id: ID, plan, marks: [], set: () => undefined, send: () => undefined };

const session = (turn: "working" | "idle"): Session =>
  ({
    id: ID,
    title: "Retry the lease",
    attachments: [],
    turn: { state: turn },
    work,
    rows: [
      { id: "m1", at: "2026-10-27T10:00:00Z", kind: "message", from: { kind: "you" }, text: "Retry the lease on a dropped socket" },
      { id: "c1", at: "2026-10-27T10:01:00Z", kind: "tool", text: "Read crates/store/src/audit.rs" },
      { id: "c2", at: "2026-10-27T10:01:10Z", kind: "tool", text: "Edit crates/store/src/audit.rs" },
    ],
  }) as unknown as Session;

function mount(one: Session, own: { work: SessionWork; ed: PlanEditing } = { work, ed }): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() =>
    root.render(
      <StoryColumn session={one} entries={[]} work={own.work} id={ID} manifestId={undefined} acts={undefined} ed={own.ed} onOpenSketch={() => undefined} onOpenSession={() => undefined} live={undefined} />,
    ),
  );
  roots.push({ root, host });
  return host;
}

const beats = (host: HTMLElement) => [...host.querySelectorAll('ul[aria-label="Now"] > li')].map((one) => one.querySelector(".sv-beat__label")?.textContent);

test("Now leads with the line the calls make, and not with a task the agent marked", () => {
  const host = mount(session("working"));
  expect(beats(host)[0]).toBe("Editing audit.rs");
  // The marked task is still in the plan under it, as a row of the working group.
  expect(host.textContent).toContain("Name the failure");
  expect(beats(host)).not.toContain("Name the failure");
});

test("with no turn running nothing is detected, and Now leads with the agent", () => {
  expect(beats(mount(session("idle")))[0]).toBe("Agent");
});

test("Whole plan opens every group where it stands, and keeps a finished one out of Now", () => {
  const three = {
    scope: [],
    approved: true,
    groups: [
      { id: "g0", title: "Read the lease", agent: "rust-engineer", paths: [], tasks: [{ id: "t0", text: "Find where it drops", state: "done" as const }] },
      plan.groups[0]!,
      { id: "g2", title: "Docs follow", agent: "docs-writer", paths: [], tasks: [{ id: "t3", text: "Say when it retries", state: "open" as const }] },
    ],
  };
  const whole = { plan: three } as unknown as SessionWork;
  const host = mount({ ...session("working"), work: whole } as unknown as Session, { work: whole, ed: { ...ed, plan: three } });
  const zone = (label: string) => host.querySelector<HTMLElement>(`section[aria-label="${label}"]`)?.textContent ?? "";
  // Folded, a finished group is a title and a bar in Done, its tasks not listed.
  expect(zone("Done")).toContain("Read the lease");
  expect(zone("Done")).not.toContain("Find where it drops");
  act(() => host.querySelector<HTMLButtonElement>('button[aria-label="Whole plan"]')!.click());
  expect(zone("Done")).toContain("Find where it drops");
  expect(zone("Now")).not.toContain("Read the lease");
  expect(zone("Now")).toContain("Retry once");
  expect(zone("Open")).toContain("Say when it retries");
});
