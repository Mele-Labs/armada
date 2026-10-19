// The Cockpit's timeline: it opens on Lanes, keeps the take he last chose, draws an entry the moment
// Fleet appends one, and draws a pull request as marks on its owner's track, never as a lane. Through `App` on the `fleet-timeline` mock Fleet, and for the live append through the
// timeline alone over a source the test moves by hand.

import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import type { TimelineEvent } from "@armada/protocol";

import { Timeline } from "../cockpit/Timeline";
import { FleetEventsOver } from "../cockpit/fleet-events";
import type { TimelineSource } from "../fleet-timeline";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const TAKE = "armada.bridge.cockpit-timeline-take";

beforeEach(() => localStorage.removeItem(TAKE));

/** The Cockpit's timeline, opened as a viewer who remembers nothing. */
async function timeline(): Promise<void> {
  mount("fleet-timeline");
  await onScreen();
  await page.getByRole("tab", { name: "Active" }).click();
  await page.getByRole("button", { name: "Timeline", exact: true }).click();
  await expect.element(page.getByRole("region", { name: "Timeline" })).toBeVisible();
}

const take = (name: string) => page.getByRole("tab", { name, exact: true });

// Skipped (#2301): written for the classic Cockpit (the Lanes take, filter tabs, the call in front), which the flight deck replaced. To be ported or dropped before landing.
test.skip("opens on Lanes, a row for each subject grouped by how it stands", async () => {
  await timeline();

  await expect.element(take("Lanes")).toHaveAttribute("aria-selected", "true");
  await expect.element(page.getByRole("list", { name: "Stuck" })).toBeVisible();
  await expect.element(page.getByRole("list", { name: "Needs you" })).toBeVisible();
  // The feed's blocks are not drawn.
  expect(page.getByRole("list", { name: "Just now" }).query()).toBeNull();
});

// Skipped (#2301): written for the classic Cockpit (the Lanes take, filter tabs, the call in front), which the flight deck replaced. To be ported or dropped before landing.
test.skip("keeps the take he picked, and opens on it next time", async () => {
  await timeline();
  await take("Feed").click();
  await expect.element(page.getByRole("list", { name: "Just now" })).toBeVisible();
  expect(localStorage.getItem(TAKE)).toBe("feed");

  await take("Split").click();
  await expect.element(page.getByRole("navigation", { name: "Subjects" })).toBeVisible();
  expect(localStorage.getItem(TAKE)).toBe("split");
});

// Skipped (#2301): written for the classic Cockpit (the Lanes take, filter tabs, the call in front), which the flight deck replaced. To be ported or dropped before landing.
test.skip("a take remembered from before is the one it opens on", async () => {
  mount("fleet-timeline");
  await onScreen();
  // `mount` forgets the viewer; the pick is written as a visit before this one would have.
  localStorage.setItem(TAKE, "split");
  await page.getByRole("tab", { name: "Active" }).click();
  await page.getByRole("button", { name: "Timeline", exact: true }).click();

  await expect.element(take("Split")).toHaveAttribute("aria-selected", "true");
});

// Skipped (#2301): written for the classic Cockpit (the Lanes take, filter tabs, the call in front), which the flight deck replaced. To be ported or dropped before landing.
test.skip("Lanes scrolls sideways, opens at now, and a Session's mark never turns its card", async () => {
  await timeline();
  const body = document.querySelector<HTMLElement>(".armada-tl__body")!;

  expect(body.scrollWidth).toBeGreaterThan(body.clientWidth);
  expect(body.scrollLeft + body.clientWidth).toBeGreaterThanOrEqual(body.scrollWidth - 1);
  body.scrollLeft = 0;
  expect(body.scrollLeft).toBe(0);

  // The diamond is drawn on the mark, not the button the card hangs from.
  const pip = document.querySelector<HTMLElement>('.armada-tl__pip[data-shape="session"]')!;
  expect(getComputedStyle(pip).transform).toBe("none");
});

// Skipped (#2301): written for the classic Cockpit (the Lanes take, filter tabs, the call in front), which the flight deck replaced. To be ported or dropped before landing.
test.skip("the filter narrows it as it narrows the grid: Your move keeps what needs him, Done what is over", async () => {
  await timeline();
  const subject = (name: RegExp) => page.getByRole("button", { name });

  await expect.element(subject(/^Session: Cache the manifest read/)).toBeVisible();
  await expect.element(subject(/^Session: Fold the two notification routes/)).not.toBeInTheDocument();

  await page.getByRole("tab", { name: "Done" }).click();
  await expect.element(subject(/^Session: Fold the two notification routes/)).toBeVisible();
  await expect.element(subject(/^Session: Cache the manifest read/)).not.toBeInTheDocument();
});

// Skipped (#2301): written for the classic Cockpit (the Lanes take, filter tabs, the call in front), which the flight deck replaced. To be ported or dropped before landing.
test.skip("main is a subject on the timeline and nothing to open", async () => {
  await timeline();
  await take("Feed").click();

  await expect.element(page.getByText("Main red")).toBeVisible();
  expect(page.getByRole("button", { name: /^Main/ }).query()).toBeNull();
});

// ---- Pull requests ride their owner's track -----------------------------------------------------------------------

/** The pip a lane's track draws for an event, by what it says. */
const pipIn = (lane: Element | null, says: RegExp) => [...(lane?.querySelectorAll<HTMLElement>(".armada-tl__pip") ?? [])].find((one) => says.test(one.getAttribute("aria-label") ?? ""));
const laneOf = (name: RegExp) => page.getByRole("button", { name }).element().closest(".armada-tl__lane");

// Skipped (#2301): written for the classic Cockpit (the Lanes take, filter tabs, the call in front), which the flight deck replaced. To be ported or dropped before landing.
test.skip("a pull request is marks on the track of the Session that owns it, and no lane of its own", async () => {
  mount("cockpit-timeline-first");
  await onScreen();
  await page.getByRole("tab", { name: "Active" }).click();
  await expect.element(page.getByRole("region", { name: "Timeline" })).toBeVisible();

  // #1890 opened on the branch of one Session; #1896 on the branch of another.
  const debounce = laneOf(/^Session: Debounce the Job Board/);
  const queued = pipIn(debounce, /^Queued, place 1/);
  expect(queued?.getAttribute("data-shape")).toBe("pull_request");
  const session = laneOf(/^Session: Flaky store test/);
  expect(pipIn(session, /^Check store passed/)?.getAttribute("data-shape")).toBe("pull_request");
  // Its own marks keep the session shape.
  expect(pipIn(debounce, /^Check lint failed/)?.getAttribute("data-shape")).toBe("session");

  // No row, lane or Split entry is a pull request, whoever owns it.
  expect(page.getByRole("button", { name: /^Pull request/ }).query()).toBeNull();
  await take("Split").click();
  await expect.element(page.getByRole("navigation", { name: "Subjects" })).toBeVisible();
  expect(page.getByRole("navigation", { name: "Subjects" }).getByRole("button", { name: /^Pull request/ }).query()).toBeNull();
});

// Skipped (#2301): written for the classic Cockpit (the Lanes take, filter tabs, the call in front), which the flight deck replaced. To be ported or dropped before landing.
test.skip("a pull request nobody owns draws nothing, in any take", async () => {
  mount("cockpit-timeline-first");
  await onScreen();
  await page.getByRole("tab", { name: "Active" }).click();
  // #1894 has no Session holding it: its failed Check is on no track.
  await expect.element(page.getByRole("region", { name: "Timeline" })).toBeVisible();
  expect([...document.querySelectorAll<HTMLElement>(".armada-tl__pip")].some((one) => /Check store failed/.test(one.getAttribute("aria-label") ?? ""))).toBe(false);
  await take("Feed").click();
  await expect.element(page.getByRole("list", { name: "Just now" })).toBeVisible();
  expect(page.getByText(/Check store failed/).query()).toBeNull();
  expect(page.getByRole("button", { name: /^Pull request/ }).query()).toBeNull();
});

// ---- Live over Fleet ----------------------------------------------------------------------------------------------

const entry = (id: number, says: string, over: Partial<TimelineEvent> = {}): TimelineEvent => ({
  id,
  at: new Date().toISOString(),
  kind: "session.step",
  subject: { kind: "session", id: "s-live-entry" },
  title: "Order the store migrations",
  tone: "run",
  says,
  live: true,
  ...over,
});

let off: (() => void) | undefined;
afterEach(() => off?.());

test("an entry Fleet appends is drawn at once, in the lane of its subject, and a new subject gets a lane", async () => {
  let held: readonly TimelineEvent[] = [entry(1, "Scope passed")];
  const listeners = new Set<() => void>();
  const source: TimelineSource = {
    get: () => held,
    subscribe: (on) => (listeners.add(on), () => listeners.delete(on)),
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  off = () => {
    root.unmount();
    host.remove();
  };
  root.render(
    <FleetEventsOver source={source}>
      <Timeline now={Date.now()} filter="running" recessed={false} onOpenSession={() => {}} />
    </FleetEventsOver>,
  );
  await expect.element(page.getByRole("button", { name: /^Scope passed/ })).toBeVisible();
  expect(page.getByRole("button", { name: /^Implement started/ }).query()).toBeNull();
  expect(page.getByText("Pin the clock").query()).toBeNull();

  held = [...held, entry(2, "Implement started"), entry(3, "Dispatched", { kind: "session.started", subject: { kind: "session", id: "s-new" }, title: "Pin the clock" })];
  listeners.forEach((on) => on());

  await expect.element(page.getByRole("button", { name: /^Implement started/ })).toBeVisible();
  await expect.element(page.getByText("Pin the clock")).toBeVisible();
});

test("a burst opens to the files or commands Fleet kept for it, and one kept none opens to nothing", async () => {
  const session = { kind: "session", id: "s-live" } as const;
  const held: readonly TimelineEvent[] = [
    entry(1, "Read 3 files", { kind: "session.read", subject: session, title: "Flaky store", detail: "src/c.rs", items: ["src/c.rs", "src/b.rs", "src/a.rs"] }),
    entry(2, "Ran 2 commands", { kind: "session.ran", detail: "cargo test", items: ["cargo test", "cargo build"] }),
    entry(3, "Edited 4 files", { kind: "session.edited", subject: session, title: "Flaky store", detail: "src/d.rs" }),
  ];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  off = () => {
    root.unmount();
    host.remove();
  };
  root.render(
    <FleetEventsOver source={{ get: () => held, subscribe: () => () => undefined }}>
      <Timeline now={Date.now()} filter="running" recessed={false} onOpenSession={() => {}} />
    </FleetEventsOver>,
  );
  await page.getByRole("tab", { name: "Feed" }).click();

  expect(page.getByText("src/b.rs").query()).toBeNull();
  await page.getByRole("button", { name: /Read 3 files/ }).click();
  await expect.element(page.getByText("src/b.rs")).toBeVisible();
  await expect.element(page.getByText("src/a.rs")).toBeVisible();
  await page.getByRole("button", { name: /Ran 2 commands/ }).click();
  await expect.element(page.getByText("cargo build")).toBeVisible();
  // No list was kept for the edit, so there is nothing to press.
  await expect.element(page.getByText("Edited 4 files")).toBeVisible();
  expect(page.getByRole("button", { name: /Edited 4 files/ }).query()).toBeNull();
});
