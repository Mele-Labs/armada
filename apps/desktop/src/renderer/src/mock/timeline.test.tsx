// The Cockpit's timeline: it opens on Lanes, keeps the take he last chose, and draws an entry the moment
// Fleet appends one. Through `App` on the `fleet-timeline` mock Fleet, and for the live append through the
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

test("opens on Lanes, a row for each subject grouped by how it stands", async () => {
  await timeline();

  await expect.element(take("Lanes")).toHaveAttribute("aria-selected", "true");
  await expect.element(page.getByRole("list", { name: "Stuck" })).toBeVisible();
  await expect.element(page.getByRole("list", { name: "Needs you" })).toBeVisible();
  // The feed's blocks are not drawn.
  expect(page.getByRole("list", { name: "Just now" }).query()).toBeNull();
});

test("keeps the take he picked, and opens on it next time", async () => {
  await timeline();
  await take("Feed").click();
  await expect.element(page.getByRole("list", { name: "Just now" })).toBeVisible();
  expect(localStorage.getItem(TAKE)).toBe("feed");

  await take("Split").click();
  await expect.element(page.getByRole("navigation", { name: "Subjects" })).toBeVisible();
  expect(localStorage.getItem(TAKE)).toBe("split");
});

test("a take remembered from before is the one it opens on", async () => {
  mount("fleet-timeline");
  await onScreen();
  // `mount` forgets the viewer; the pick is written as a visit before this one would have.
  localStorage.setItem(TAKE, "split");
  await page.getByRole("tab", { name: "Active" }).click();
  await page.getByRole("button", { name: "Timeline", exact: true }).click();

  await expect.element(take("Split")).toHaveAttribute("aria-selected", "true");
});

test("Lanes scrolls sideways, opens at now, and a Session's mark never turns its card", async () => {
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

test("the filter narrows it as it narrows the grid: Your move keeps what needs him, Done what is over", async () => {
  await timeline();
  const subject = (name: RegExp) => page.getByRole("button", { name });

  await expect.element(subject(/^Job: Cache the manifest read/)).toBeVisible();
  await expect.element(subject(/^Job: Fold the two notification routes/)).not.toBeInTheDocument();

  await page.getByRole("tab", { name: "Done" }).click();
  await expect.element(subject(/^Job: Fold the two notification routes/)).toBeVisible();
  await expect.element(subject(/^Job: Cache the manifest read/)).not.toBeInTheDocument();
});

test("main is a subject on the timeline and nothing to open", async () => {
  await timeline();
  await take("Feed").click();

  await expect.element(page.getByText("Main red")).toBeVisible();
  expect(page.getByRole("button", { name: /^Main/ }).query()).toBeNull();
});

// ---- Live over Fleet ----------------------------------------------------------------------------------------------

const entry = (id: number, says: string, over: Partial<TimelineEvent> = {}): TimelineEvent => ({
  id,
  at: new Date().toISOString(),
  kind: "job.step",
  subject: { kind: "job", id: "job-live" },
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
      <Timeline now={Date.now()} jobs={[]} filter="running" recessed={false} onOpenJob={() => {}} onOpenSession={() => {}} onOpenLink={() => {}} />
    </FleetEventsOver>,
  );
  await expect.element(page.getByRole("button", { name: /^Scope passed/ })).toBeVisible();
  expect(page.getByRole("button", { name: /^Implement started/ }).query()).toBeNull();
  expect(page.getByText("#1900 Pin the clock").query()).toBeNull();

  held = [...held, entry(2, "Implement started"), entry(3, "Opened", { kind: "pr.opened", subject: { kind: "pull_request", number: 1900 }, title: "Pin the clock" })];
  listeners.forEach((on) => on());

  await expect.element(page.getByRole("button", { name: /^Implement started/ })).toBeVisible();
  await expect.element(page.getByText("#1900 Pin the clock")).toBeVisible();
});
