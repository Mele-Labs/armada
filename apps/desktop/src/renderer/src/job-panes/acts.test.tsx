// A Job's acts in a Thread head that has too little room: the name keeps its 18 characters, the acts that do not fit are rows of the ellipsis menu and Enter on one runs it,
// and an act that is on or running says so on its glyph, in the head and in its row. Browser: the head's width is what is read, so the head is laid out for real.
// The Job is drawn with no file of the owner's, so the acts are the built-in ones.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeAll, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { SessionFrame, SessionToolbelt } from "@armada/components";
import type { FlightControls } from "@armada/protocol";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../../shared/bridge";
import { NOTHING_YET } from "../../../shared/bridge";
import { SessionTiles } from "../cockpit/deck/SessionTiles";
import { JobActsSlot, actEntries, headTake, runActEntry, useActs, useActsPlace } from "./acts";
import "../styles/index.css";
import "../cockpit/deck/deck.css";
import "./job-panes.css";

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

const NONE: FlightControls = {
  version: 1,
  job: [],
  deck: [],
};

let base: unknown;
beforeAll(() => {
  const state = {
    ...NOTHING_YET,
    connection: {
      state: "connected",
    },
  } as unknown as BridgeState;
  base = (
    window as unknown as {
      armada: unknown;
    }
  ).armada = {
    subscribeChanges: () => () => {},
    syncState: async () => ({
      full: state,
      seq: 1,
    }),
    getFlightControls: async () => ({
      ok: true,
      value: {
        what: "read",
        controls: NONE,
      },
    }),
    saveFlightControls: async (controls: FlightControls) => ({
      ok: true,
      value: {
        what: "read",
        controls,
      },
    }),
    getRepositoryFlightControls: async (repository: string) => ({
      ok: true,
      value: {
        repository,
        place: "this_mac",
        read: {
          what: "read",
          controls: {
            version: 1,
            job: [],
            deck: [],
          },
        },
      },
    }),
  };
});

const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];
/** Takes the head away: the acts and where they stand are one Job's, so a second head waits for the first to go. */
const leave = () => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
};
afterEach(() => {
  leave();
  delete document.documentElement.dataset.headTake;
  (
    window as unknown as {
      armada: unknown;
    }
  ).armada = base;
});

const flush = () => act(async () => void (await new Promise((done) => setTimeout(done, 0))));

const TITLE = "Retry the slot lease when two Jobs ask for the same bay at once";
const REPOSITORY = {
  root: "/code/armada",
  name: "armada",
};
const jobOf = (more: object = {}): Session =>
  ({
    id: "s1",
    title: TITLE,
    rows: [],
    attachments: [],
    turn: {
      state: "idle",
    },
    ...more,
  }) as unknown as Session;
const draftOf = (more: object = {}) =>
  ({
    commands: [],
    get: () => [],
    send: vi.fn(async () => true),
    autoLand: vi.fn(async () => true),
    pause: vi.fn(async () => true),
    resume: vi.fn(async () => true),
    ...more,
  }) as unknown as SessionsDraft & {
    autoLand: ReturnType<typeof vi.fn>;
  };

/** A Thread head as the Cockpit draws one: the frame, the slot the acts stand in, and the menu that takes the ones with no room. */
function Head({
  session,
  draft,
  under = false,
}: {
  session: Session;
  draft: SessionsDraft;
  under?: boolean;
}) {
  const acts = useActs();
  const { away } = useActsPlace();
  return (
    <>
      <SessionFrame
        state="quiet"
        said="Quiet"
        id={session.id}
        {...(session.title === undefined
          ? {}
          : {
              title: session.title,
            })}
        headTake={headTake()}
        {...(under
          ? {
              toolsUnder: true,
            }
          : {})}
        actions={<JobActsSlot />}
        menu={{
          entries: actEntries(acts, away),
          onSelect: (id) => void runActEntry(acts, id),
        }}
        back={<button type="button" aria-label="Close" />}
      >
        {under ? (
          <>
            <div data-composer="" />
            <SessionToolbelt />
          </>
        ) : null}
      </SessionFrame>
      <SessionTiles head session={session} draft={draft} repository={REPOSITORY} />
    </>
  );
}

/** The head, `width` wide, laid out. */
async function mount(
  width: number,
  session: Session = jobOf(),
  draft: SessionsDraft = draftOf(),
  under = false,
) {
  const host = document.createElement("div");
  host.style.cssText = `width: ${width}px; display: flex; flex-direction: column`;
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Head session={session} draft={draft} under={under} />));
  roots.push({
    root,
    host,
  });
  // Each measuring is a step, and the row settles in as many as it has acts.
  for (let i = 0; i < 12; i++) await flush();
  return host;
}

const head = (host: HTMLElement) => host.querySelector<HTMLElement>(".armada-session-frame__head")!;
const drawn = (host: HTMLElement) =>
  [
    ...host.querySelectorAll<HTMLElement>('.job-acts[role="toolbar"] > *'),
  ].map((one) => (one.querySelector("button") ?? one).getAttribute("aria-label"));
const icon = (host: HTMLElement, name: string) =>
  host.querySelector<HTMLElement>(`.job-acts[role="toolbar"] button[aria-label="${name}"]`)!;

/** The name's width against 18 characters of its own type. */
function nameAgainstFloor(host: HTMLElement) {
  const title = host.querySelector<HTMLElement>(".armada-session-frame__title")!;
  const probe = document.createElement("span");
  probe.style.cssText = "position:absolute;visibility:hidden;width:18ch";
  title.append(probe);
  const floor = probe.getBoundingClientRect().width;
  probe.remove();
  return {
    name: host
      .querySelector<HTMLElement>(".armada-session-frame__title-text")!
      .getBoundingClientRect().width,
    floor,
  };
}

for (const take of [
  "rows",
  "overflow",
] as const) {
  test(`${take}: the name keeps 18 characters at a narrow width, and the head does not overflow`, async () => {
    if (take === "overflow") document.documentElement.dataset.headTake = "overflow";
    const host = await mount(360);
    const { name, floor } = nameAgainstFloor(host);
    expect(floor).toBeGreaterThan(0);
    expect(name).toBeGreaterThanOrEqual(floor);
    expect(head(host).scrollWidth).toBeLessThanOrEqual(head(host).clientWidth + 1);
    for (const row of host.querySelectorAll<HTMLElement>(".armada-session-frame__row"))
      expect(row.scrollWidth).toBeLessThanOrEqual(row.clientWidth + 1);
  });
}

test("every act is an icon where the row has room, and none is in the menu", async () => {
  const host = await mount(900);
  expect(drawn(host)).toEqual([
    "Auto land",
    "Pause",
    "Judge",
    "Report issue with job",
    "Release everything the Job holds",
    "Requests",
    "Checks",
    "Subagents",
  ]);
});

test("the acts with no room leave in a fixed order, the one read at a glance last", async () => {
  // At 170 some have left and some stay: what stays is the last of the order they leave in, whichever the head draws first.
  const order = [
    "Requests",
    "Release everything the Job holds",
    "Report issue with job",
    "Subagents",
    "Judge",
    "Checks",
    "Auto land",
    "Pause",
  ];
  const kept = drawn(await mount(170));
  expect(kept.length).toBeGreaterThan(0);
  expect(kept.length).toBeLessThan(order.length);
  expect(
    [
      ...kept,
    ].sort(),
  ).toEqual(order.slice(order.length - kept.length).sort());
});

test("an act with no room is a row of the ellipsis menu, and Enter on the row runs it", async () => {
  const draft = draftOf();
  const host = await mount(120, jobOf(), draft);
  expect(drawn(host)).not.toContain("Auto land");
  host.querySelector<HTMLElement>('button[aria-label="More"]')!.focus();
  await userEvent.keyboard("{Enter}");
  await flush();
  const rows = () => [
    ...document.querySelectorAll<HTMLElement>('[role="menuitem"]'),
  ];
  expect(rows().map((one) => one.textContent)).toContain("Auto land");
  // Arrow to the row by its name, as a keyboard does, and press Enter.
  for (let i = 0; i < rows().length && document.activeElement?.textContent !== "Auto land"; i++)
    await userEvent.keyboard("{ArrowDown}");
  expect(document.activeElement?.textContent).toBe("Auto land");
  await userEvent.keyboard("{Enter}");
  await flush();
  expect(draft.autoLand.mock.calls).toEqual([
    [
      "s1",
      true,
    ],
  ]);
});

test("a switch that is on carries its state and says it in its tooltip, and an off one does not", async () => {
  const on = await mount(
    900,
    jobOf({
      autoLand: true,
    }),
  );
  expect(icon(on, "Auto land").dataset.on).toBe("true");
  expect(icon(on, "Pause").hasAttribute("data-on")).toBe(false);
  await userEvent.hover(icon(on, "Auto land"));
  await expect.element(page.getByText("Auto land on")).toBeVisible();
  await userEvent.unhover(icon(on, "Auto land"));
  leave();
  const off = await mount(900, jobOf());
  expect(icon(off, "Auto land").hasAttribute("data-on")).toBe(false);
});

test("a paused Job's Pause is on", async () => {
  const host = await mount(
    900,
    jobOf({
      paused: true,
    }),
  );
  expect(icon(host, "Resume").dataset.on).toBe("true");
});

test("a thing that is running carries its state and says it in its tooltip, and stops when nothing runs", async () => {
  const running = [
    {
      kind: "subagent",
      id: "a1",
      task: "Read the retry path",
      state: "running",
    },
  ];
  const host = await mount(
    900,
    jobOf({
      attachments: running,
    }),
  );
  expect(icon(host, "Subagents").dataset.running).toBe("true");
  await userEvent.hover(icon(host, "Subagents"));
  await expect.element(page.getByText("Subagents running")).toBeVisible();
  await userEvent.unhover(icon(host, "Subagents"));
  leave();
  const done = await mount(
    900,
    jobOf({
      attachments: [
        {
          ...running[0],
          state: "done",
        },
      ],
    }),
  );
  expect(icon(done, "Subagents").hasAttribute("data-running")).toBe(false);
});

test("a running Check carries its state", async () => {
  const host = await mount(
    900,
    jobOf({
      rows: [
        {
          id: "c1",
          at: "14:00:00",
          kind: "check",
          name: "tests",
          state: "running",
        },
      ],
    }),
  );
  expect(icon(host, "Checks").dataset.running).toBe("true");
});

test("an act in the menu carries its state on its row", async () => {
  const running = [
    {
      kind: "subagent",
      id: "a1",
      task: "Read the retry path",
      state: "running",
    },
  ];
  const host = await mount(
    120,
    jobOf({
      autoLand: true,
      attachments: running,
    }),
  );
  host.querySelector<HTMLElement>('button[aria-label="More"]')!.focus();
  await userEvent.keyboard("{Enter}");
  await flush();
  const row = (name: string) =>
    [
      ...document.querySelectorAll<HTMLElement>('[role="menuitem"]'),
    ].find((one) => one.textContent?.startsWith(name));
  expect(row("Auto land")?.dataset.state).toBe("on");
  expect(row("Subagents")?.dataset.state).toBe("running");
  // Judge has left too, with nothing on it.
  expect(row("Judge")).toBeDefined();
  expect(row("Judge")?.dataset.state).toBeUndefined();
});

test("under the composer, the toolbelt follows it and the head keeps the name, the menu and the way out", async () => {
  const host = await mount(900, jobOf(), draftOf(), true);
  const composer = host.querySelector<HTMLElement>("[data-composer]")!;
  const belt = host.querySelector<HTMLElement>(".armada-session-frame__toolbelt")!;
  expect(composer.compareDocumentPosition(belt) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(belt.querySelectorAll('.job-acts[role="toolbar"]').length).toBe(1);
  expect(head(host).querySelector(".job-acts")).toBeNull();
  expect(head(host).querySelector('[aria-label="Close"]')).not.toBeNull();
  expect(head(host).querySelector(".armada-session-frame__title")).not.toBeNull();
});
