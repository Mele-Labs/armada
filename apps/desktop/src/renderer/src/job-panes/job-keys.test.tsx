// A Job in panes has the story's Done, Now and Open, and the views its keys open (Artifacts, Checks, Subagents, Bays, System, Workflow), as tabs. A key brings that kind's tab forward, or opens one where the Job has none, and never
// opens a layer over the Job or a second tab. Mounted as the Cockpit mounts it: the Job's workspace with a place for its story, over the real panes.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { Session, SessionAttachment, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { SessionsFrom } from "../sessions-draft";
import { closeLayer, layerOver, showLayer, toggleLayer } from "../session-view/layers";
import { toggleLanes } from "../session-view/lanes-width";
import type { PlanEditing } from "../session-view/PlanEdit";
import { SessionWorkspace } from "../session-view/SessionWorkspace";
import { resetFolds } from "../session-view/story";
import { endZoom, toggleZoom } from "./zoom";
import { forgetPanes, peek, rearrange } from "./store";
import { addTab, holderOf, panesOf } from "./tree";

// The mod list is Fleet's, read through `window.armada`; no Job here made a mod.
vi.mock("../mod-list", () => ({
  useModList: () => [],
}));

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

// The views read Fleet through `window.armada`, which holds an empty state and answers that it has nothing else.
const named: Record<string, unknown> = {
  syncState: async () => ({
    full: {
      connection: {
        state: "disconnected",
      },
    },
    seq: 1,
  }),
  subscribeChanges: () => () => undefined,
  getSessionSystem: async () => ({
    ok: false,
    said: "",
  }),
};
const armada = new Proxy(named, {
  get: (them, name) =>
    them[String(name)] ??
    (String(name).startsWith("on") ? () => () => undefined : async () => undefined),
});
const ID = "keys-s1";
const roots: {
  root: Root;
  host: HTMLElement;
  side: HTMLElement;
}[] = [];
beforeEach(() => {
  forgetPanes();
  (
    window as unknown as {
      armada: unknown;
    }
  ).armada = armada;
});
afterEach(() => {
  closeLayer();
  resetFolds(ID);
  endZoom();
  localStorage.clear();
  for (const { root, host, side } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
    side.remove();
  }
});

const attachments: SessionAttachment[] = [
  {
    kind: "pull_request",
    number: 7,
    title: "Retry the lease",
    branch: "armada/retry",
    address: "https://forge/7",
    checks: {
      state: "passed",
    },
    state: "draft",
    auto: false,
  },
  {
    kind: "artifact",
    form: "page",
    id: "https://notes/lease",
    title: "Lease notes",
  },
];
const plan = {
  scope: [],
  approved: true,
  groups: [
    {
      id: "g1",
      title: "Retry the lease",
      agent: "rust-engineer",
      paths: [],
      tasks: [
        {
          id: "t1",
          text: "Retry once",
          state: "open" as const,
        },
      ],
    },
  ],
};
const session = {
  id: ID,
  title: "Retry the lease",
  attachments,
  rows: [],
  turn: {
    state: "idle",
  },
  work: {
    plan,
  },
} as unknown as Session;
const ed: PlanEditing = {
  id: ID,
  plan,
  marks: [],
  set: () => undefined,
  send: () => undefined,
};
const held = [
  session,
];

function mount() {
  const host = document.createElement("div");
  host.style.cssText = "position:fixed;top:0;left:0;width:1400px;height:800px";
  const side = document.createElement("div");
  side.className = "sv-side";
  document.body.append(host, side);
  const draft = {
    get: () => held,
    subscribe: () => () => undefined,
    work: {
      workflows: [],
    },
    commands: [],
  } as unknown as SessionsDraft;
  const root = createRoot(host);
  act(() =>
    root.render(
      <SessionsFrom held={draft}>
        <SessionWorkspace
          session={session}
          entries={[]}
          repositories={[]}
          hosts={{
            side,
          }}
          ed={ed}
          bays={undefined}
          onOpenSketch={() => undefined}
          onOpenSession={() => undefined}
          onOpenCheck={() => undefined}
        >
          <div data-thread="" />
        </SessionWorkspace>
      </SessionsFrom>,
    ),
  );
  roots.push({
    root,
    host,
    side,
  });
  const unmount = () => {
    act(() => root.unmount());
    host.remove();
    side.remove();
    roots.splice(
      roots.findIndex((one) => one.root === root),
      1,
    );
  };
  return {
    host,
    unmount,
  };
}

const press = (init: KeyboardEventInit) =>
  act(
    () =>
      void window.dispatchEvent(
        new KeyboardEvent("keydown", {
          bubbles: true,
          cancelable: true,
          ...init,
        }),
      ),
  );
const tabs = () =>
  [
    ...document.querySelectorAll('[role="tab"]'),
  ].map((one) => one.getAttribute("aria-label") ?? one.textContent?.trim() ?? "");
const kinds = () => panesOf(peek(ID).root).flatMap((one) => one.tabs.map((tab) => tab.kind));
const front = (kind: string) =>
  panesOf(peek(ID).root).some((one) =>
    one.tabs.some((tab) => tab.id === one.active && tab.kind === kind),
  );
// A view drawn as a layer over the Job, with the head and Back of one.
const layers = () => document.querySelectorAll("section.sv-layer:not([data-in-pane])");

test("the story is the Done, Now and Open tabs in order, with no right column or layer", () => {
  mount();
  const all = tabs().map((one) => one.toLowerCase());
  const at = (name: string) => all.findIndex((one) => one.includes(name));
  expect(at("done")).toBeGreaterThanOrEqual(0);
  expect(at("done")).toBeLessThan(at("now"));
  expect(at("now")).toBeLessThan(at("open"));
  expect(kinds()).toEqual(
    expect.arrayContaining([
      "thread",
      "done",
      "now",
      "open",
    ]),
  );
  expect(layers()).toHaveLength(0);
});

test("⇧⌘A brings the Ledger tab forward, or opens one, and again opens neither a second nor a layer", () => {
  mount();
  press({
    metaKey: true,
    shiftKey: true,
    code: "KeyA",
  });
  expect(kinds().filter((one) => one === "ledger")).toHaveLength(1);
  expect(front("ledger")).toBe(true);
  expect(layers()).toHaveLength(0);
  expect(layerOver(ID)).toBeUndefined();

  // Another tab in front of it, in its own pane, and the key brings it back without opening a second.
  const pane = holderOf(peek(ID), "ledger")!;
  expect(pane.tabs.length).toBeGreaterThan(0);
  press({
    metaKey: true,
    shiftKey: true,
    code: "KeyA",
  });
  expect(kinds().filter((one) => one === "ledger")).toHaveLength(1);
  expect(front("ledger")).toBe(true);
  // Esc is nobody's Back: the tab stays.
  press({
    key: "Escape",
  });
  expect(kinds()).toContain("ledger");
});

test("a band's key brings its tab forward: ⌥1 Done, ⌥2 Now, ⌥3 Open", () => {
  mount();
  for (const [code, kind] of [
    [
      "Digit1",
      "done",
    ],
    [
      "Digit2",
      "now",
    ],
    [
      "Digit3",
      "open",
    ],
  ] as const) {
    // A Files tab opens in front of the tab, in its own pane, so the key has something to bring back.
    act(() =>
      rearrange(ID, (arrangement) => addTab(arrangement, holderOf(arrangement, kind)!.id, "files")),
    );
    expect(front(kind)).toBe(false);
    press({
      altKey: true,
      code,
    });
    expect(front(kind)).toBe(true);
    expect(kinds().filter((one) => one === kind)).toHaveLength(1);
  }
  expect(layers()).toHaveLength(0);
});

test("Checks, Subagents, Bays, System, Workflow and Judge open as tabs, by the calls their keys and acts make, and open no layer", () => {
  mount();
  toggleLayer(ID, "checks");
  toggleLayer(ID, "subagents");
  showLayer(ID, "slot");
  toggleLayer(ID, "system");
  toggleLanes(ID);
  showLayer(ID, "judge");
  for (const kind of [
    "checks",
    "subagents",
    "bays",
    "system",
    "workflow",
    "judge",
  ]) {
    expect(
      kinds().filter((one) => one === kind),
      kind,
    ).toHaveLength(1);
  }
  expect(layerOver(ID)).toBeUndefined();
  expect(layers()).toHaveLength(0);

  // A second press brings the tab forward and does not close it, since a tab is closed by its ×.
  const was = kinds().length;
  toggleLayer(ID, "checks");
  expect(kinds()).toHaveLength(was);
  expect(front("checks")).toBe(true);
});

test("a tab the Job opened is still there when the Job is left and entered again, and a zoom is not", () => {
  const first = mount();
  press({
    metaKey: true,
    shiftKey: true,
    code: "KeyA",
  });
  const holder = holderOf(peek(ID), "ledger")!;
  act(() => toggleZoom(ID, holder.id));
  first.unmount();
  mount();
  expect(kinds()).toContain("ledger");
  expect(front("ledger")).toBe(true);
  expect(document.querySelector("[data-zoomed]")).toBeNull();
});

test("the views are tabs a pane offers under +, and a view's tab has no Back and no Esc of its own", () => {
  mount();
  const add = document.querySelector<HTMLElement>('button[aria-label="Add tab"]')!;
  act(() =>
    add.dispatchEvent(
      new PointerEvent("pointerdown", {
        bubbles: true,
        button: 0,
      }),
    ),
  );
  act(() => add.click());
  const items = [
    ...document.querySelectorAll('[role="menuitem"]'),
  ].map((one) => one.textContent?.trim());
  for (const name of [
    "Ledger",
    "Checks",
    "Subagents",
    "System",
    "Bays",
  ])
    expect(items, name).toContain(name);

  toggleLayer(ID, "checks");
  const checks = document.querySelector('section.sv-layer[aria-label="Checks"]');
  expect(checks).not.toBeNull();
  expect(checks!.hasAttribute("data-in-pane")).toBe(true);
  expect(checks!.querySelector('button[aria-label="Back"]')).toBeNull();
  press({
    key: "Escape",
  });
  expect(document.querySelector('section.sv-layer[aria-label="Checks"]')).not.toBeNull();
});
