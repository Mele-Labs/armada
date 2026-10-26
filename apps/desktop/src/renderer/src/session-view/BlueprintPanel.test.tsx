// The Blueprint in the Job's right column: a drafted group ends in an empty task to type into, and an approved group that works an issue opens that issue
// from its title, with the issue on a meta line of its own. Browser: the drawing and its presses are what is read.

import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { PlanMark, Session, SessionAttachment, WorkGroup } from "@armada/screens/src/draft/sessions";

import { openInPanel } from "../shared-panel/store";
import { BlueprintPanel } from "./BlueprintPanel";
import type { PlanEditing } from "./PlanEdit";
import { resetFolds } from "./story";

vi.mock("../shared-panel/store", async (original) => ({ ...(await original<typeof import("../shared-panel/store")>()), openInPanel: vi.fn(() => true) }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  resetFolds(ID);
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  vi.mocked(openInPanel).mockClear();
});

const ID = "blueprint-s1";
const issue: SessionAttachment = { kind: "issue", number: "2212", title: "Retry the lease on a dropped socket", address: "https://forge/issues/2212", state: "open" };

const groups = (named?: string): WorkGroup[] => [
  { id: "g1", title: "Retry the lease", agent: "rust-engineer", paths: ["crates/ipc"], ...(named === undefined ? {} : { issue: named }), tasks: [{ id: "t1", text: "Name the failure", state: "done" }, { id: "t2", text: "Retry once", state: "open" }] },
  { id: "g2", title: "Tell the person", agent: "bridge-engineer", paths: [], tasks: [{ id: "t3", text: "Show the retry", state: "open" }] },
];

function sessionOf(approved: boolean, named?: string): Session {
  return { id: ID, attachments: [issue], rows: [], turn: { state: "idle" }, work: { plan: { scope: [], approved, groups: groups(named) } } } as unknown as Session;
}

/** The marks live in the test, as they do in the Session's view: the panel is given the plan, the marks and the way to change them. */
function Mounted({ session, seen }: { session: Session; seen?: (marks: readonly PlanMark[]) => void }) {
  const [marks, setMarks] = useState<PlanMark[]>([]);
  const plan = session.work!.plan!;
  const ed: PlanEditing = {
    id: session.id,
    plan,
    marks,
    set: (next) => (seen?.(next), setMarks(next)),
    send: () => undefined,
    ...(plan.approved ? {} : { approve: () => undefined }),
  };
  return <BlueprintPanel work={session.work!} place="side" id={session.id} session={session} manifestId="armada" acts={undefined} ed={ed} onOpenSketch={() => undefined} onOpenSession={() => undefined} />;
}

function mount(session: Session, seen?: (marks: readonly PlanMark[]) => void): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Mounted session={session} {...(seen === undefined ? {} : { seen })} />));
  roots.push({ root, host });
  return host;
}

const change = (field: HTMLInputElement, value: string) =>
  act(() => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(field, value);
    field.dispatchEvent(new Event("input", { bubbles: true }));
  });
const key = (field: HTMLElement, name: string) => act(() => void field.dispatchEvent(new KeyboardEvent("keydown", { key: name, bubbles: true, cancelable: true })));
const section = (host: HTMLElement, group: string) => host.querySelector<HTMLElement>(`section[data-group="${group}"]`)!;
const newTask = (host: HTMLElement, group: string) => section(host, group).querySelector<HTMLInputElement>(`input[aria-label="New task in ${groups().find((one) => one.id === group)!.title}"]`)!;

test("every drafted group ends in an empty task line to type into", () => {
  const host = mount(sessionOf(false));
  for (const id of ["g1", "g2"]) {
    const lines = [...section(host, id).querySelectorAll("ul.sv-tasks > li")];
    const last = lines.at(-1)!;
    expect(last.classList.contains("sv-task--new")).toBe(true);
    expect(last.querySelector("input")?.value).toBe("");
  }
});

test("typing a task and pressing Enter adds a task mark and leaves the line empty for the next", () => {
  const seen: (readonly PlanMark[])[] = [];
  const host = mount(sessionOf(false), (marks) => seen.push(marks));
  const field = newTask(host, "g1");
  change(field, "  Log the retry  ");
  expect(field.value).toBe("  Log the retry  ");
  key(field, "Enter");
  expect(seen).toHaveLength(1);
  expect(seen[0]).toEqual([expect.objectContaining({ kind: "add", group: "g1", text: "Log the retry" })]);
  // The task stands in the group, marked as added and not sent, and the line under it is empty and still the last.
  expect(section(host, "g1").querySelector('.sv-task--edit [aria-label="Task: Log the retry"]')).not.toBeNull();
  expect(section(host, "g1").querySelector('.sv-task--edit [aria-label="Added"]')).not.toBeNull();
  expect(newTask(host, "g1").value).toBe("");
  expect(section(host, "g1").querySelector("ul.sv-tasks > li:last-child")?.classList.contains("sv-task--new")).toBe(true);
  // The other group's line is its own.
  expect(newTask(host, "g2").value).toBe("");
});

test("Enter on an empty or blank line adds nothing", () => {
  const seen: (readonly PlanMark[])[] = [];
  const host = mount(sessionOf(false), (marks) => seen.push(marks));
  const field = newTask(host, "g1");
  key(field, "Enter");
  change(field, "   ");
  key(field, "Enter");
  expect(seen).toEqual([]);
});

test("Esc empties the line without adding a task", () => {
  const seen: (readonly PlanMark[])[] = [];
  const host = mount(sessionOf(false), (marks) => seen.push(marks));
  const field = newTask(host, "g1");
  change(field, "Something half typed");
  expect(field.value).toBe("Something half typed");
  key(field, "Escape");
  expect(newTask(host, "g1").value).toBe("");
  expect(seen).toEqual([]);
});

// ---- An approved group that works an issue -------------------------------------------------------------------

/** An approved plan shows the Now view; the whole plan is a press on the band's button away. */
function whole(host: HTMLElement) {
  act(() => host.querySelector<HTMLElement>('button[aria-label="Whole blueprint"]')!.click());
}

test("an approved group's title opens its issue in the side panel, with the Manifest", () => {
  const host = mount(sessionOf(true, "2212"));
  whole(host);
  // The title is the row's first press; its tooltip text rides inside the button, so the title is its first text.
  const title = section(host, "g1").querySelector<HTMLElement>(".sv-group__row > button.sv-group__title")!;
  expect(title.firstChild?.textContent).toBe("Retry the lease");
  act(() => title.click());
  expect(openInPanel).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({ kind: "issue", number: "2212", manifestId: "armada" }));
});

test("the chevron of a group that works an issue opens and closes the group, and opens no issue", () => {
  const host = mount(sessionOf(true, "2212"));
  whole(host);
  const chevron = section(host, "g1").querySelector<HTMLElement>('button[aria-label="Subagent type of Retry the lease"]')!;
  expect(chevron.getAttribute("aria-expanded")).toBe("false");
  expect(section(host, "g1").querySelector(".sv-expand")).toBeNull();
  act(() => chevron.click());
  expect(chevron.getAttribute("aria-expanded")).toBe("true");
  expect(section(host, "g1").querySelector(".sv-expand")).not.toBeNull();
  act(() => chevron.click());
  expect(chevron.getAttribute("aria-expanded")).toBe("false");
  expect(section(host, "g1").querySelector(".sv-expand")).toBeNull();
  expect(openInPanel).not.toHaveBeenCalled();
});

test("a group that works an issue draws it on a meta line of its own, with its key and its title", () => {
  const host = mount(sessionOf(true, "2212"));
  whole(host);
  const meta = section(host, "g1").querySelector(".sv-group__meta")!;
  expect(meta.querySelector(".sv-issue__key")?.textContent).toBe("#2212");
  expect(meta.querySelector(".sv-issue__title")?.textContent).toBe("Retry the lease on a dropped socket");
  // The meta line is not the title's line: the group's row holds the title and the chevron only.
  expect(section(host, "g1").querySelector(".sv-group__row")?.contains(meta)).toBe(false);
  act(() => meta.querySelector<HTMLElement>("button")!.click());
  expect(openInPanel).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({ kind: "issue", number: "2212" }));
});

test("a group that names no issue has no meta line, and its row is the one press that opens it", () => {
  const host = mount(sessionOf(true));
  whole(host);
  expect(section(host, "g1").querySelector(".sv-group__meta")).toBeNull();
  const row = section(host, "g1").querySelector<HTMLElement>("button.sv-group__row")!;
  expect(row.getAttribute("aria-expanded")).toBe("false");
  act(() => row.click());
  expect(row.getAttribute("aria-expanded")).toBe("true");
  expect(openInPanel).not.toHaveBeenCalled();
});

test("a group naming an issue the Job does not hold draws no meta line", () => {
  const host = mount(sessionOf(true, "9999"));
  whole(host);
  expect(section(host, "g1").querySelector(".sv-group__meta")).toBeNull();
});
