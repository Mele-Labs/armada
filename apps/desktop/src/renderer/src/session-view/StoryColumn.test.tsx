// What leads Now in the Job's story: the line worked out from the Job's tool calls, ahead of any task the agent has marked, and the agent by name when nothing is
// being done. Browser: the drawing is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { Session, SessionWork } from "@armada/screens/src/draft/sessions";

import type { PlanEditing } from "./PlanEdit";
import { labelOf } from "../job-panes/PaneView";
import { forgetPanes, peek } from "../job-panes/store";
import { panesOf } from "../job-panes/tree";
import { drawLayersAsTabs } from "./layers";
import { StoryColumn, type StoryZone } from "./StoryColumn";
import { resetFolds } from "./story";

// The mod list is Fleet's, read through `window.armada`; no Job here made a mod.
vi.mock("../mod-list", () => ({
  useModList: () => [],
}));
// The Jobs this one started are read from the window's Sessions.
const held: {
  sessions: Session[];
} = {
  sessions: [],
};
vi.mock("../sessions-draft", async (original) => ({
  ...(await original<typeof import("../sessions-draft")>()),
  useSessions: () => held.sessions,
}));

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

const ID = "story-column-s1";
const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];
afterEach(() => {
  held.sessions = [];
  resetFolds(ID);
  localStorage.clear();
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

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
          text: "Name the failure",
          state: "working" as const,
        },
        {
          id: "t2",
          text: "Retry once",
          state: "open" as const,
        },
      ],
    },
  ],
};
const work = {
  plan,
} as unknown as SessionWork;
const ed: PlanEditing = {
  id: ID,
  plan,
  marks: [],
  set: () => undefined,
  send: () => undefined,
};

const session = (turn: "working" | "idle"): Session =>
  ({
    id: ID,
    title: "Retry the lease",
    attachments: [],
    turn: {
      state: turn,
    },
    work,
    rows: [
      {
        id: "m1",
        at: "2026-10-27T10:00:00Z",
        kind: "message",
        from: {
          kind: "you",
        },
        text: "Retry the lease on a dropped socket",
      },
      {
        id: "c1",
        at: "2026-10-27T10:01:00Z",
        kind: "tool",
        text: "Read crates/store/src/audit.rs",
      },
      {
        id: "c2",
        at: "2026-10-27T10:01:10Z",
        kind: "tool",
        text: "Edit crates/store/src/audit.rs",
      },
    ],
  }) as unknown as Session;

function mount(
  one: Session,
  own: {
    work: SessionWork;
    ed: PlanEditing;
  } = {
    work,
    ed,
  },
  only?: StoryZone,
): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() =>
    root.render(
      <StoryColumn
        session={one}
        only={only}
        entries={[]}
        work={own.work}
        id={ID}
        manifestId={undefined}
        acts={undefined}
        ed={own.ed}
        onOpenSketch={() => undefined}
        onOpenSession={() => undefined}
        live={undefined}
      />,
    ),
  );
  roots.push({
    root,
    host,
  });
  return host;
}

const beats = (host: HTMLElement) =>
  [
    ...host.querySelectorAll('ul[aria-label="Now"] > li'),
  ].map((one) => one.querySelector(".sv-beat__label")?.textContent);

test("Now leads with the line the calls make, and not with a task the agent marked", () => {
  const host = mount(session("working"));
  expect(beats(host)[0]).toBe("Editing audit.rs");
  // The marked task is still in the plan under it, as a row of the working group.
  expect(host.textContent).toContain("Name the failure");
  expect(beats(host)).not.toContain("Name the failure");
});

test("with no turn running nothing is detected, and Now draws no agent row", () => {
  expect(beats(mount(session("idle")))).not.toContain("Agent");
});

test("with no calls to read, an agent held on a question for him is named for what it waits on", () => {
  const held = {
    ...session("idle"),
    asked: {
      questions: [
        {
          question: "Keep the rotation at midnight?",
        },
      ],
    },
  } as unknown as Session;
  expect(beats(mount(held))).toContain("Waiting on you");
  expect(beats(mount(held))).not.toContain("Agent");
});

test("Whole plan opens every group where it stands, and keeps a finished one out of Now", () => {
  const three = {
    scope: [],
    approved: true,
    groups: [
      {
        id: "g0",
        title: "Read the lease",
        agent: "rust-engineer",
        paths: [],
        tasks: [
          {
            id: "t0",
            text: "Find where it drops",
            state: "done" as const,
          },
        ],
      },
      plan.groups[0]!,
      {
        id: "g2",
        title: "Docs follow",
        agent: "docs-writer",
        paths: [],
        tasks: [
          {
            id: "t3",
            text: "Say when it retries",
            state: "open" as const,
          },
        ],
      },
    ],
  };
  const whole = {
    plan: three,
  } as unknown as SessionWork;
  const host = mount(
    {
      ...session("working"),
      work: whole,
    } as unknown as Session,
    {
      work: whole,
      ed: {
        ...ed,
        plan: three,
      },
    },
  );
  const zone = (label: string) =>
    host.querySelector<HTMLElement>(`section[aria-label="${label}"]`)?.textContent ?? "";
  // Folded, a finished group is a title and a bar in Done, its tasks not listed.
  expect(zone("Done")).toContain("Read the lease");
  expect(zone("Done")).not.toContain("Find where it drops");
  act(() => host.querySelector<HTMLButtonElement>('button[aria-label="Whole plan"]')!.click());
  expect(zone("Done")).toContain("Find where it drops");
  expect(zone("Now")).not.toContain("Read the lease");
  expect(zone("Now")).toContain("Retry once");
  expect(zone("Open")).toContain("Say when it retries");
});

test("a walk shown again at one address is one beat in Now, the newest showing", () => {
  const url = "http://localhost:5511/?walk=processes";
  const again = session("idle");
  const shown = {
    ...again,
    rows: [
      ...again.rows,
      {
        id: "w1",
        at: "2026-10-27T10:02:00Z",
        kind: "window",
        title: "Processes",
        url,
      },
      {
        id: "w2",
        at: "2026-10-27T10:03:00Z",
        kind: "window",
        title: "Processes, take 2",
        url,
      },
    ],
  } as unknown as Session;
  const host = mount(shown);
  const walks = [
    ...host.querySelectorAll('ul[aria-label="Now"] > li[data-walk]'),
  ].map((one) => one.querySelector(".sv-beat__label")?.textContent);
  expect(walks).toEqual([
    "Processes, take 2",
  ]);
});

const childJob = (id: string, title: string, change: object): Session =>
  ({
    id,
    title,
    attachments: [
      {
        kind: "started_by",
        id: ID,
      },
    ],
    rows: [],
    turn: {
      state: "idle",
    },
    standing: "idle",
    ...change,
  }) as unknown as Session;
const zoneOf = (host: HTMLElement, label: string) =>
  host.querySelector<HTMLElement>(`section[aria-label="${label}"]`);
const cardsIn = (zone: HTMLElement | null) =>
  [
    ...(zone?.querySelectorAll("button.armada-job-card") ?? []),
  ].map((one) => one.getAttribute("aria-label"));

test("a Job started outside the plan stands in Now while it runs and in Done once it has ended", () => {
  const running = childJob("c1", "Seed the test database", {
    turn: {
      state: "working",
    },
    standing: "working",
  });
  const ended = childJob("c2", "Check an old dump restores", {
    dead: "ended",
  });
  const parent = {
    ...session("idle"),
    attachments: [
      {
        kind: "started_to",
        id: "c1",
      },
      {
        kind: "started_to",
        id: "c2",
      },
    ],
  } as unknown as Session;
  held.sessions = [
    parent,
    running,
    ended,
  ];
  const host = mount(parent);
  expect(cardsIn(zoneOf(host, "Now"))).toEqual([
    "Job Seed the test database",
  ]);
  expect(cardsIn(zoneOf(host, "Done"))).toEqual([
    "Job Check an old dump restores",
  ]);
});

test("a group handed to a Job still running stands in Now though the parent ticked every task when it handed it", () => {
  const ticked = {
    scope: [],
    approved: true,
    groups: [
      {
        id: "g1",
        title: "Name the migrations",
        agent: "rust-engineer",
        paths: [],
        tasks: [
          {
            id: "t1",
            text: "Hand it over",
            state: "done" as const,
          },
        ],
      },
    ],
  };
  const own = {
    work: {
      plan: ticked,
    } as unknown as SessionWork,
    ed: {
      ...ed,
      plan: ticked,
    },
  };
  const parent = {
    ...session("idle"),
    work: own.work,
    attachments: [
      {
        kind: "started_to",
        id: "c1",
        group: "g1",
      },
    ],
  } as unknown as Session;
  held.sessions = [
    parent,
    childJob("c1", "Migrations get names", {
      turn: {
        state: "working",
      },
      standing: "working",
    }),
  ];
  const host = mount(parent, own);
  expect(cardsIn(zoneOf(host, "Now"))).toEqual([
    "Job Migrations get names",
  ]);
  expect(cardsIn(zoneOf(host, "Done"))).toEqual([]);
});

const pullRequest = (number: number, title: string, state: "draft" | "open" | "merged") => ({
  kind: "pull_request",
  number,
  title,
  branch: `armada/change-${number}`,
  address: `https://git.example/pull/${number}`,
  checks: {
    state: "passed",
  },
  state,
  auto: false,
});
const ticket = (number: string, title: string, state: "open" | "closed") => ({
  kind: "issue",
  number,
  title,
  address: `https://tracker.example/browse/${number}`,
  state,
});

test("a draft pull request and an open ticket stand in Open, and Done holds only the merged and the closed", () => {
  const holding = {
    ...session("idle"),
    attachments: [
      ticket("ABC-12", "Default rate is missing from the stack", "open"),
      ticket("ABC-9", "Rate picker shows a stale par rate", "closed"),
      pullRequest(41, "Let leverage options submit a below-par rate", "draft"),
      pullRequest(40, "Read the par rate from the stack", "merged"),
    ],
  } as unknown as Session;
  const host = mount(holding);
  const done = zoneOf(host, "Done")?.textContent ?? "";
  expect(done).not.toContain("Let leverage options submit a below-par rate");
  expect(done).not.toContain("Default rate is missing from the stack");
  expect(done).toContain("Read the par rate from the stack");
  expect(done).toContain("Rate picker shows a stale par rate");
  const open = zoneOf(host, "Open")?.textContent ?? "";
  expect(open).toContain("Let leverage options submit a below-par rate");
  expect(open).toContain("Default rate is missing from the stack");
  expect(open).not.toContain("Read the par rate from the stack");
  const now = zoneOf(host, "Now")?.textContent ?? "";
  expect(now).not.toContain("Let leverage options submit a below-par rate");
  expect(now).not.toContain("Default rate is missing from the stack");
});

test("the Waiting on you line for a plain draft opens its card as a tab on the Job, and pressing it again opens no second", () => {
  forgetPanes();
  const release = drawLayersAsTabs(ID, () => false);
  const title = "Description for the queue pull request";
  const asked = {
    ...session("idle"),
    rows: [
      {
        id: "d1",
        at: "2026-10-27T10:02:00Z",
        kind: "draft",
        title,
        text: "Retries the lease once.",
      },
    ],
    waitingFor: [
      {
        id: "draft:d1",
        text: title,
        since: "2026-10-27T10:02:00Z",
        source: "draft",
        act: {
          kind: "answer",
          target: "draft:d1",
        },
        options: [
          {
            label: "Approve",
          },
        ],
      },
    ],
  } as unknown as Session;
  const host = mount(asked);
  const line = () =>
    host.querySelector<HTMLButtonElement>('ul[aria-label="Now"] button.sv-beat__open');
  expect(line()?.textContent, "the line is pressable").toBe(title);
  act(() => line()!.click());
  act(() => line()!.click());
  release();
  const tabs = panesOf(peek(ID).root)
    .flatMap((pane) => pane.tabs)
    .filter((tab) => tab.kind === "draft");
  expect(tabs.map((tab) => labelOf(ID, tab))).toEqual([
    title,
  ]);
});

const keeping = {
  ...session("idle"),
  later: [
    {
      id: "l1",
      text: "Rotate the audit key",
      at: "2026-10-27T10:03:00Z",
    },
  ],
} as unknown as Session;

test("a Deferred pane draws the follow-ups kept for later, and an Open pane does not", () => {
  const deferred = mount(keeping, undefined, "deferred");
  expect(deferred.querySelector('[data-area="deferred"] section.sv-later')).not.toBeNull();
  const open = mount(keeping, undefined, "next");
  expect(open.querySelector("section.sv-later")).toBeNull();
  expect(open.querySelector('[data-area="deferred"]')).toBeNull();
});

test("a pane's one zone is marked single and draws no zone-name label, while the whole story keeps the spine and its labels", () => {
  const alone = mount(keeping, undefined, "now");
  expect(alone.querySelector(".sv-spine")?.hasAttribute("data-single")).toBe(true);
  expect(alone.querySelector(".sv-panel__label")).toBeNull();
  const all = mount(keeping);
  expect(all.querySelector(".sv-spine")?.hasAttribute("data-single")).toBe(false);
  for (const zone of [
    "Now",
    "Open",
  ]) {
    expect(
      all.querySelector(`section[aria-label="${zone}"] .sv-panel__label`)?.textContent,
    ).toContain(zone);
  }
});

// A Job on a workflow, at Walk: Propose is finished, Refine and Deliver are still to come.
const FLOW = [
  "Propose",
  "Walk",
  "Refine",
  "Deliver",
];
const later = {
  id: "g2",
  title: "Later work",
  agent: "docs-writer",
  paths: [],
  tasks: [
    {
      id: "t3",
      text: "Say when it retries",
      state: "open" as const,
    },
  ],
};
const onFlow = (off = false) => {
  const planned = {
    ...plan,
    groups: [
      plan.groups[0]!,
      later,
    ],
  };
  const flowing = {
    plan: planned,
    ...(off
      ? {
          railOff: true,
        }
      : {}),
    workflow: {
      name: "Product",
      steps: FLOW,
      at: 1,
    },
  } as unknown as SessionWork;
  return {
    one: {
      ...session("idle"),
      work: flowing,
    } as unknown as Session,
    own: {
      work: flowing,
      ed: {
        ...ed,
        plan: planned,
      },
    },
  };
};
const panels = (host: HTMLElement) =>
  [
    ...host.querySelectorAll("section.sv-panel"),
  ].map((one) => one.getAttribute("aria-label"));

test("on a workflow, a pane for Now holds the step it is at and the steps still to come, and a pane for Open holds neither", () => {
  const { one, own } = onFlow();
  const now = mount(one, own, "now");
  expect(panels(now)).toEqual([
    "Walk",
    "Refine",
    "Deliver",
  ]);
  const open = mount(one, own, "next");
  expect(panels(open)).not.toContain("Refine");
  expect(panels(open)).not.toContain("Deliver");
  expect(open.textContent).not.toContain("Pull request");
});

test("with the workflow off, a pane for Now lists the steps and route still to come, and a pane for Open lists only what is open", () => {
  const { one, own } = onFlow(true);
  const now = mount(one, own, "now");
  const ahead = now.querySelector('section[aria-label="Now"] ol[aria-label="Ahead"]')!;
  expect(ahead.textContent).toContain("Refine");
  expect(ahead.textContent).toContain("Deliver");
  const open = mount(one, own, "next");
  expect(open.querySelector('ol[aria-label="Ahead"]')).toBeNull();
  expect(open.textContent).toContain("Later work");
  expect(open.textContent).not.toContain("Refine");
});

test("a finished step in a pane of its own keeps its mark in the row beside its name", () => {
  const { one, own } = onFlow();
  const done = mount(one, own, "done");
  const propose = done.querySelector<HTMLElement>('section[aria-label="Propose"]')!;
  const band = propose.querySelector(".sv-panel__band")!;
  const mark = band.querySelector<HTMLElement>('[role="img"][aria-label="Done"]')!;
  expect(mark, "the mark stands in the band with the name").not.toBeNull();
  expect(band.querySelector(".sv-panel__label")?.textContent).toContain("Propose");
  expect(getComputedStyle(mark).position).toBe("static");
  // On the spine the same mark hangs on the line, out of the row.
  const spine = mount(one, own);
  const online = spine.querySelector<HTMLElement>(
    'section[aria-label="Propose"] [role="img"][aria-label="Done"]',
  )!;
  expect(getComputedStyle(online).position).toBe("absolute");
});

test("on a workflow, Open lists an open ticket and a draft pull request until they land", () => {
  const { one, own } = onFlow();
  const withOpen = {
    ...one,
    attachments: [
      ...one.attachments,
      {
        kind: "issue",
        number: "2936",
        title: "Document the audit log rotation",
        address: "https://git.example/armada/issues/2936",
        state: "open",
      },
      {
        kind: "pull_request",
        number: 2940,
        title: "Keep the rotation window in the config",
        branch: "armada/audit-rotation-window",
        address: "https://git.example/armada/pull/2940",
        checks: {
          state: "running",
        },
        state: "draft",
        auto: false,
        closes: [],
      },
    ],
  } as unknown as Session;
  const open = mount(withOpen, own, "next");
  expect(open.textContent).toContain("Document the audit log rotation");
  expect(open.textContent).toContain("Keep the rotation window in the config");
});
