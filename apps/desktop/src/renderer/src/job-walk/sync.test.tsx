// A Job's tabs for the walks it owes him, kept by `WalkTabsSync`: real attachments give tabs, a settled walk takes its tab, a tab closed with its × stays owed and comes back when he
// opens the Job, and a walk pressed open comes to the front.

import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";
import { createRoot, type Root } from "react-dom/client";
import { act } from "react";
import { afterEach, beforeEach, expect, test } from "vitest";

import { forgetDeck, select } from "../cockpit/deck/store";
import { forgetPanes, peek, rearrange } from "../job-panes/store";
import { closeTab, panesOf } from "../job-panes/tree";
import { SessionsFrom } from "../sessions-draft";
import { bringWalkForward, forgetWalks } from "./store";
import { WalkTabsSync } from "./sync";

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

const BILLING = "http://localhost:5394/?walk=billing";
const SETTINGS = "http://localhost:5394/?walk=settings";
const ACCOUNT = "http://localhost:5394/?walk=account";
const TITLES: Record<string, string> = {
  [BILLING]: "Billing page",
  [SETTINGS]: "Settings page",
  [ACCOUNT]: "Account page",
};

const walkOf = (id: string, title: string) =>
  ({
    kind: "artifact",
    form: "window",
    id,
    title,
    window: {
      takes: [],
      steps: [],
    },
  }) as const;
const waitingOn = (id: string, title: string) => ({
  id: `walk:${id}`,
  text: title,
  since: "2026-11-03T10:00:00Z",
  source: "walk" as const,
  act: {
    kind: "walk" as const,
    target: id,
  },
});

/** A Job that showed the walks named; each is owed while `owed` holds its address. */
const job = (owed: readonly string[]): Session =>
  ({
    id: "job",
    title: "Billing",
    turn: {
      state: "idle",
    },
    rows: [],
    attachments: Object.entries(TITLES).map(([id, title]) => walkOf(id, title)),
    waitingFor: owed.map((id) => waitingOn(id, TITLES[id]!)),
  }) as unknown as Session;

let sessions: readonly Session[] = [];
const hearers = new Set<() => void>();
const held = {
  get: () => sessions,
  subscribe: (hear: () => void) => {
    hearers.add(hear);
    return () => hearers.delete(hear);
  },
} as unknown as SessionsDraft;
const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];

const change = (next: readonly Session[]) =>
  act(async () => {
    sessions = next;
    hearers.forEach((hear) => hear());
  });

const tabs = () =>
  panesOf(peek("job").root).flatMap((pane) => pane.tabs.filter((tab) => tab.kind === "walk"));
const front = () => {
  const pane = panesOf(peek("job").root).find((one) =>
    one.tabs.some((tab) => tab.kind === "walk"),
  )!;
  return pane.tabs.find((tab) => tab.id === pane.active)!;
};

async function draw(here = true) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({
    root,
    host,
  });
  await act(async () =>
    root.render(
      <SessionsFrom held={held}>
        <WalkTabsSync here={here} />
      </SessionsFrom>,
    ),
  );
}

beforeEach(() => {
  forgetPanes();
  forgetWalks();
  forgetDeck();
  sessions = [];
});
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

test("a real walk the Job shows gets a tab named by its page, and the settled one takes it away", async () => {
  sessions = [
    job([
      BILLING,
    ]),
  ];
  await draw();
  expect(
    tabs().map((tab) => [
      tab.label,
      tab.page,
    ]),
  ).toEqual([
    [
      "Billing page",
      BILLING,
    ],
  ]);
  // Approved: Fleet dropped the item. The walk is still in the Job's record.
  await change([
    job([]),
  ]);
  expect(tabs()).toEqual([]);
});

test("a walk shown while he is in another Job comes to the front of its panes, and in the Job he is in it arrives behind", async () => {
  sessions = [
    job([
      BILLING,
    ]),
  ];
  await draw();
  await change([
    job([
      BILLING,
      SETTINGS,
    ]),
  ]);
  expect(front().page).toBe(SETTINGS);
  select({
    kind: "session",
    key: "session:job",
  });
  await change([
    job([
      BILLING,
      SETTINGS,
      ACCOUNT,
    ]),
  ]);
  expect(tabs().map((tab) => tab.page)).toEqual([
    BILLING,
    SETTINGS,
    ACCOUNT,
  ]);
  expect(front().page).toBe(SETTINGS);
});

test("a tab closed with its × is not made again by a change in the walks, and opening the Job brings it back in front", async () => {
  sessions = [
    job([
      BILLING,
    ]),
  ];
  await draw();
  rearrange("job", (arrangement) => closeTab(arrangement, tabs()[0]!.id));
  expect(tabs()).toEqual([]);
  await change([
    job([
      BILLING,
    ]),
  ]);
  expect(tabs()).toEqual([]);
  select({
    kind: "session",
    key: "session:job",
  });
  await change([
    job([
      BILLING,
    ]),
  ]);
  expect(tabs().map((tab) => tab.page)).toEqual([
    BILLING,
  ]);
});

test("a walk pressed open comes to the front, and one that has no tab is given one", async () => {
  sessions = [
    job([
      BILLING,
      SETTINGS,
    ]),
  ];
  await draw();
  expect(tabs()).toHaveLength(2);
  const first = tabs().find((tab) => tab.page === BILLING)!;
  await act(async () => bringWalkForward("job", BILLING));
  expect(front().id).toBe(first.id);
  rearrange("job", (arrangement) =>
    closeTab(arrangement, tabs().find((tab) => tab.page === SETTINGS)!.id),
  );
  await act(async () => bringWalkForward("job", SETTINGS));
  expect(front().page).toBe(SETTINGS);
});
