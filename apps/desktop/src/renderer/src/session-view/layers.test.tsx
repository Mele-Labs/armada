// The layers over a Session's thread: the store that opens and closes them, and the four layers, read off a rendering. Fully keyboard driven:
// focus goes in, the arrows walk the rows, Esc is Back, and Back gives focus to what had it. Browser: a rendering is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { LedgerEntry } from "@armada/components";
import type { WorktreeSlot, WriteClash } from "@armada/protocol";
import type { Session, SessionAttachment, SessionRow } from "@armada/screens/src/draft/sessions";

import { closeLayer, dropLayer, showLayer, toggleLayer, useLayer } from "./layers";
import { Layer } from "./SessionLayers";
import type { SessionBays } from "./slot-trouble";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  closeLayer();
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const ID = "s1";
const attachments: SessionAttachment[] = [
  { kind: "pull_request", number: 7, title: "Retry the lease", branch: "armada/retry", address: "https://forge/7", checks: { state: "passed" }, state: "open", auto: false },
  { kind: "artifact", form: "page", id: "https://notes/lease", title: "Lease notes" },
  { kind: "artifact", form: "file", id: "docs/other.md", title: "Other" },
  { kind: "subagent", id: "sa-1", task: "Find callers", state: "done", agentType: "Explore" },
  { kind: "subagent", id: "sa-2", task: "Write the wire", state: "running", agentType: "protocol-engineer" },
];
const rows: SessionRow[] = [
  { id: "c1", at: "13:00:00", kind: "check", name: "fleet", run: 11, state: "passed" },
  { id: "c2", at: "13:05:00", kind: "check", name: "desktop", run: 12, state: "running" },
];
const session = { id: ID, title: "Retry the lease", attachments, rows, turn: { state: "idle" } } as unknown as Session;

const entries = [
  { key: "https://notes/lease", kind: "artifact", artifact: "page", name: "Lease notes", text: "Lease notes", onOpen: () => undefined },
  { key: "docs/other.md", kind: "artifact", artifact: "file", name: "Other", text: "Other", onOpen: () => undefined },
] as unknown as LedgerEntry[];

const slot = { slot: 3, manifest_id: "m", path: "/s/3", base: "main", warm: true, behind: 31, branch: "armada/retry", held: { state: "stranded", why: "2 uncommitted" }, stranded: { uncommitted: ["a.rs"], commits: [], unpushed: 1 } } as unknown as WorktreeSlot;
const other = { slot: 5, manifest_id: "m", path: "/s/5", base: "main", warm: true, behind: 2, branch: "armada/retry-wire", held: { state: "hosted_session", session_id: ID } } as unknown as WorktreeSlot;
const clash = { one: { kind: "session", id: ID, label: "Retry the lease" }, other: { kind: "session", id: "s2", label: "Name the migrations" }, paths: ["crates/ipc/src/lib.rs"], since: "2026-10-17T13:00:00Z" } satisfies WriteClash;
const bays: SessionBays = {
  bays: [
    { slot, branch: "armada/retry", wrong: "stranded", said: "Slot 3 is stranded" },
    { slot: other, branch: "armada/retry-wire", wrong: undefined, said: undefined },
  ],
  clashes: [clash],
};

function Open({ over, checked }: { over: "artifacts" | "subagents" | "checks" | "slot"; checked?: (run: number) => void }) {
  const layer = useLayer(ID);
  return layer === undefined ? null : <Layer id={layer} session={session} entries={entries} bays={over === "slot" ? bays : undefined} onOpenCheck={checked ?? (() => undefined)} onOpenSession={() => undefined} />;
}

function mount(over: "artifacts" | "subagents" | "checks" | "slot", checked?: (run: number) => void): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Open over={over} {...(checked === undefined ? {} : { checked })} />));
  roots.push({ root, host });
  return host;
}

const layer = (host: HTMLElement) => host.querySelector<HTMLElement>("section.sv-layer");
const key = (init: KeyboardEventInit) => act(() => void document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init })));
const rowsOf = (host: HTMLElement) => [...host.querySelectorAll<HTMLElement>("[data-layer-row]")].map((one) => one.getAttribute("aria-label"));

test("the same key again closes the layer, a different key swaps it, and dropping the Session closes it", () => {
  const host = mount("artifacts");
  expect(layer(host)).toBeNull();
  act(() => toggleLayer(ID, "artifacts"));
  expect(layer(host)?.getAttribute("aria-label")).toBe("Artifacts");
  act(() => toggleLayer(ID, "subagents"));
  expect(layer(host)?.getAttribute("aria-label")).toBe("Subagents");
  act(() => toggleLayer(ID, "subagents"));
  expect(layer(host)).toBeNull();
  act(() => showLayer(ID, "checks"));
  act(() => dropLayer(ID));
  expect(layer(host)).toBeNull();
});

test("Back gives focus to what had it before the first layer opened", async () => {
  const button = document.createElement("button");
  document.body.append(button);
  button.focus();
  const host = mount("artifacts");
  act(() => toggleLayer(ID, "artifacts"));
  act(() => toggleLayer(ID, "checks"));
  expect(document.activeElement).not.toBe(button);
  act(() => closeLayer());
  await vi.waitFor(() => expect(document.activeElement).toBe(button));
  button.remove();
  expect(layer(host)).toBeNull();
});

test("Esc is Back from wherever focus is in the layer", () => {
  const host = mount("artifacts");
  act(() => showLayer(ID, "artifacts"));
  expect(layer(host)).not.toBeNull();
  key({ key: "Escape" });
  expect(layer(host)).toBeNull();
});

test("Artifacts opens with focus in its search field, narrows as it is typed in, and the arrows walk the rows", () => {
  const host = mount("artifacts");
  act(() => showLayer(ID, "artifacts"));
  const field = host.querySelector<HTMLInputElement>('[aria-label="Search artifacts"]')!;
  expect(document.activeElement).toBe(field);
  expect(host.querySelectorAll(".sv-layer__group").length).toBeGreaterThan(2);
  key({ key: "ArrowDown" });
  expect(document.activeElement?.getAttribute("aria-label")).toBe("Pull request #7 Retry the lease");
  key({ key: "ArrowDown" });
  expect(document.activeElement?.getAttribute("aria-label")).toBe("Lease notes");
  key({ key: "ArrowDown" });
  expect(document.activeElement?.getAttribute("aria-label")).toBe("Other");
  key({ key: "ArrowUp" });
  key({ key: "ArrowUp" });
  key({ key: "ArrowUp" });
  expect(document.activeElement).toBe(field);
  // Typing narrows the groups at once.
  act(() => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(field, "notes");
    field.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect([...host.querySelectorAll(".sv-layer__group-label")].map((one) => one.textContent)).toEqual(["Pages"]);
});

test("Commands lists the ten newest, and View all shows the rest", () => {
  const ran = Array.from({ length: 12 }, (_, n): SessionRow => ({ id: `r${n}`, at: "13:01:00", kind: "ran", command: `echo ${n}`, code: 0, tail: "" }));
  rows.push(...ran);
  try {
    const host = mount("artifacts");
    act(() => showLayer(ID, "artifacts"));
    const commands = () => host.querySelectorAll('[aria-label="Commands"] .sv-ledger').length;
    expect(commands()).toBe(10);
    const more = [...host.querySelectorAll<HTMLButtonElement>('[aria-label="Commands"] button')].find((one) => one.textContent === "View all 12")!;
    act(() => more.click());
    expect(commands()).toBe(12);
    expect(host.querySelector('[aria-label="Commands"] [aria-expanded="true"]')?.textContent).toBe("Show fewer");
  } finally {
    rows.splice(rows.length - ran.length, ran.length);
  }
});

test("Subagents lists every one, the running one first, and each is a row to press", () => {
  const host = mount("subagents");
  act(() => showLayer(ID, "subagents"));
  expect(layer(host)?.getAttribute("aria-label")).toBe("Subagents");
  const titles = [...host.querySelectorAll(".sv-ledger__title")].map((one) => one.textContent);
  expect(titles).toEqual(["Write the wire", "Find callers"]);
});

test("Checks lists the runs newest first and a press opens the run", () => {
  const opened: number[] = [];
  const host = mount("checks", (run) => opened.push(run));
  act(() => showLayer(ID, "checks"));
  expect(rowsOf(host)).toEqual(["desktop, running", "fleet, passed"]);
  expect(document.activeElement?.getAttribute("aria-label")).toBe("desktop, running");
  act(() => (document.activeElement as HTMLElement).click());
  expect(opened).toEqual([12]);
});

test("Bays lists the write clashes first, then every bay the Session holds drawn as the cockpit draws it, and offers each one", () => {
  const host = mount("slot");
  act(() => showLayer(ID, "slot"));
  const groups = [...host.querySelectorAll(".sv-layer__group")].map((one) => one.getAttribute("aria-label"));
  expect(groups).toEqual(["Write clashes", "slot-3", "slot-5"]);
  const marks = (name: string) => [...host.querySelectorAll(`section[aria-label="${name}"] [role="img"]`)].map((one) => one.getAttribute("aria-label"));
  const text = (name: string) => host.querySelector(`section[aria-label="${name}"]`)?.textContent ?? "";
  expect(marks("slot-3")).toEqual(expect.arrayContaining(["Stranded", "1 commit not pushed", "31 behind main"]));
  expect(marks("slot-5")).toEqual(expect.arrayContaining(["2 behind main"]));
  expect(text("slot-5")).toContain("armada/retry-wire");
  const clashRows = [...host.querySelectorAll('table[aria-label="Write clashes"] tbody tr')].map((row) => [...row.querySelectorAll("td")].map((cell) => cell.textContent));
  expect(clashRows).toEqual([[expect.stringContaining("Name the migrations"), "crates/ipc/src/lib.rs"]]);
  expect(host.querySelector('button[aria-label="Open Name the migrations"]')).not.toBeNull();
  expect([...host.querySelectorAll("[data-layer-row]")].map((one) => one.getAttribute("aria-label"))).toEqual(["Show slot-3 in Bays", "Show slot-5 in Bays"]);
});
