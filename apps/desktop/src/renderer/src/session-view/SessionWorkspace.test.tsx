// The Job's right column answers its keys: ⇧⌘A opens Artifacts over the column, where the story is, and Esc gives the story back. Browser: the workspace is
// mounted over a host standing for the deck's right column, and a rendering is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import type { Session, SessionAttachment, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { SessionsFrom } from "../sessions-draft";
import { closeLayer } from "./layers";
import type { PlanEditing } from "./PlanEdit";
import { SessionWorkspace } from "./SessionWorkspace";
import { resetFolds } from "./story";

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

// A draft pull request is still being built, so the Field is the stage and the Landing stands folded.
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
  const draft = { get: () => held, subscribe: () => () => undefined, work: {} } as unknown as SessionsDraft;
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
const panels = (side: HTMLElement) => [...side.querySelectorAll(":scope > section.sv-panel")].map((one) => one.getAttribute("data-area"));

test("the story is the right column's Blueprint, Field and Landing in order, with no layer over it", () => {
  const { side } = mount();
  expect(panels(side)).toEqual(["blueprint", "field", "landing"]);
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
  expect(panels(side)).toEqual(["blueprint", "field", "landing"]);
});

test("⇧⌘A again closes Artifacts", () => {
  const { side } = mount();
  press({ metaKey: true, shiftKey: true, code: "KeyA" });
  expect(layer(side)).not.toBeNull();
  press({ metaKey: true, shiftKey: true, code: "KeyA" });
  expect(layer(side)).toBeNull();
});

test("a band's key while Artifacts is open closes it and folds the band by hand", () => {
  const { side } = mount();
  const landing = () => side.querySelector('section[data-area="landing"]')!;
  expect(landing().hasAttribute("data-collapsed")).toBe(true);
  press({ metaKey: true, shiftKey: true, code: "KeyA" });
  press({ altKey: true, code: "KeyL" });
  expect(layer(side)).toBeNull();
  expect(landing().hasAttribute("data-collapsed")).toBe(false);
});

test("a panel opened by hand is still open when the Job is left and entered again", () => {
  const first = mount();
  press({ altKey: true, code: "KeyL" });
  expect(first.side.querySelector('section[data-area="landing"]')!.hasAttribute("data-collapsed")).toBe(false);
  first.unmount();
  const again = mount();
  expect(again.side.querySelector('section[data-area="landing"]')!.hasAttribute("data-collapsed")).toBe(false);
});

test("the keys other than ⇧⌘A open nothing: ⌘A alone and ⇧A alone leave the story", () => {
  const { side } = mount();
  press({ metaKey: true, code: "KeyA" });
  press({ shiftKey: true, code: "KeyA" });
  expect(layer(side)).toBeNull();
});
