// The Job's right column answers its keys: ⇧⌘A opens Artifacts over the column, where the story is, and Esc gives the story back. Browser: the workspace is
// mounted over a host standing for the deck's right column, and a rendering is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { Session, SessionAttachment, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { SessionsFrom } from "../sessions-draft";
import { closeLayer } from "./layers";
import type { PlanEditing } from "./PlanEdit";
import { SessionWorkspace } from "./SessionWorkspace";
import { resetFolds } from "./story";

// The mod list is Fleet's, read through `window.armada`; no Job here made a mod.
vi.mock("../mod-list", () => ({ useModList: () => [] }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const ID = "workspace-s1";
const roots: { root: Root; host: HTMLElement; side: HTMLElement }[] = [];
afterEach(() => {
  closeLayer();
  resetFolds(ID);
  localStorage.clear();
  for (const { root, host, side } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
    side.remove();
  }
});

// A draft pull request is still being built. Every zone of the story is open, whatever the stage.
const attachments: SessionAttachment[] = [
  { kind: "pull_request", number: 7, title: "Retry the lease", branch: "armada/retry", address: "https://forge/7", checks: { state: "passed" }, state: "draft", auto: false },
  { kind: "artifact", form: "page", id: "https://notes/lease", title: "Lease notes" },
];
const plan = { scope: [], approved: true, groups: [{ id: "g1", title: "Retry the lease", agent: "rust-engineer", paths: [], tasks: [{ id: "t1", text: "Retry once", state: "open" as const }] }] };
const session = { id: ID, title: "Retry the lease", attachments, rows: [], turn: { state: "idle" }, work: { plan } } as unknown as Session;
const ed: PlanEditing = { id: ID, plan, marks: [], set: () => undefined, send: () => undefined };

const held = [session];

function mount() {
  const thread = document.createElement("div");
  const side = document.createElement("div");
  side.className = "sv-side";
  document.body.append(thread, side);
  const draft = { get: () => held, subscribe: () => () => undefined, work: { workflows: [] }, commands: [] } as unknown as SessionsDraft;
  const root = createRoot(thread);
  act(() =>
    root.render(
      <SessionsFrom held={draft}>
        <SessionWorkspace session={session} entries={[]} repositories={[]} hosts={{ side }} ed={ed} bays={undefined} onOpenSketch={() => undefined} onOpenSession={() => undefined} onOpenCheck={() => undefined}>
          <div data-thread="" />
        </SessionWorkspace>
      </SessionsFrom>,
    ),
  );
  roots.push({ root, host: thread, side });
  const unmount = () => {
    act(() => root.unmount());
    thread.remove();
    side.remove();
    roots.splice(roots.findIndex((one) => one.root === root), 1);
  };
  return { thread, side, unmount };
}

const press = (init: KeyboardEventInit) => act(() => void window.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init })));
const layer = (side: HTMLElement) => side.querySelector('section.sv-layer[aria-label="Artifacts"]');
const panels = (side: HTMLElement) => [...side.querySelectorAll("section.sv-panel")].map((one) => one.getAttribute("data-area"));

test("the story is the right column's Completed, Now and Next in order on one timeline, with no layer over it", () => {
  const { side } = mount();
  expect(panels(side)).toEqual(["done", "now", "next"]);
  expect(side.querySelector(".sv-spine")).not.toBeNull();
  expect(layer(side)).toBeNull();
});

test("⇧⌘A opens Artifacts over the right column, and Esc gives the story back", () => {
  const { side, thread } = mount();
  press({ metaKey: true, shiftKey: true, code: "KeyA" });
  const open = layer(side);
  expect(open).not.toBeNull();
  // It covers the column, not the thread: the thread's host holds no layer.
  expect(thread.querySelector("section.sv-layer")).toBeNull();
  expect(open!.textContent).toContain("Lease notes");
  press({ key: "Escape" });
  expect(layer(side)).toBeNull();
  expect(panels(side)).toEqual(["done", "now", "next"]);
});

test("⇧⌘A again closes Artifacts", () => {
  const { side } = mount();
  press({ metaKey: true, shiftKey: true, code: "KeyA" });
  expect(layer(side)).not.toBeNull();
  press({ metaKey: true, shiftKey: true, code: "KeyA" });
  expect(layer(side)).toBeNull();
});

test("a band's key while Artifacts is open closes it and folds the zone by hand", () => {
  const { side } = mount();
  const next = () => side.querySelector('section[data-area="next"]')!;
  expect(next().hasAttribute("data-collapsed")).toBe(false);
  press({ metaKey: true, shiftKey: true, code: "KeyA" });
  press({ altKey: true, code: "Digit3" });
  expect(layer(side)).toBeNull();
  expect(next().hasAttribute("data-collapsed")).toBe(true);
});

test("a zone folded by hand is still folded when the Job is left and entered again", () => {
  const first = mount();
  press({ altKey: true, code: "Digit3" });
  expect(first.side.querySelector('section[data-area="next"]')!.hasAttribute("data-collapsed")).toBe(true);
  first.unmount();
  const again = mount();
  expect(again.side.querySelector('section[data-area="next"]')!.hasAttribute("data-collapsed")).toBe(true);
});

test("⌥M puts the caret in the open Job's Message box", () => {
  const { thread } = mount();
  const dock = document.createElement("div");
  dock.className = "sv-dock";
  const box = document.createElement("div");
  box.setAttribute("role", "textbox");
  box.setAttribute("aria-label", "Message");
  box.contentEditable = "true";
  box.tabIndex = 0;
  dock.append(box);
  thread.append(dock);
  expect(document.activeElement).not.toBe(box);
  press({ altKey: true, code: "KeyM", key: "µ" });
  expect(document.activeElement).toBe(box);
});

test("the keys other than ⇧⌘A open nothing: ⌘A alone and ⇧A alone leave the story", () => {
  const { side } = mount();
  press({ metaKey: true, code: "KeyA" });
  press({ shiftKey: true, code: "KeyA" });
  expect(layer(side)).toBeNull();
});

test("the four views are a strip above the story, and a press opens a view in the spine under it, and a view with nothing in it stays off", () => {
  const { side, thread } = mount();
  const strip = side.querySelector('[role="toolbar"][aria-label="Views"]');
  expect(strip).not.toBeNull();
  expect([...strip!.querySelectorAll("button[data-view]")].map((one) => one.getAttribute("data-view"))).toEqual(["artifacts", "checks", "subagents", "slot"]);
  // This Job has run no Checks, so its button is in its place and off.
  expect(strip!.querySelector<HTMLButtonElement>('button[data-view="checks"]')!.disabled).toBe(true);
  act(() => strip!.querySelector<HTMLButtonElement>('button[data-view="artifacts"]')!.click());
  const open = side.querySelector('section.sv-layer[aria-label="Artifacts"]');
  expect(open).not.toBeNull();
  // In the spine's body, beside the story, with the strip outside it; the thread's host holds no layer.
  expect(open!.closest(".sv-story-body")).not.toBeNull();
  expect(strip!.closest(".sv-story-body")).toBeNull();
  expect(thread.querySelector("section.sv-layer")).toBeNull();
  act(() => strip!.querySelector<HTMLButtonElement>('button[data-view="artifacts"]')!.click());
  expect(side.querySelector("section.sv-layer")).toBeNull();
});
