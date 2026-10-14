// The Timeline's feed on its own, which the flight deck mounts in a narrow column: a press on a row picks the
// event rather than opening its subject, the deck's own filter narrows the lanes, and the row it holds lit says
// so. Over a source the test hands in, with no `App` and no mock Fleet.

import type { TimelineEvent } from "@armada/protocol";
import { createRoot } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "../styles/index.css";
import { ActivityFeed } from "./Timeline";
import { FleetEventsOver, type FleetEvent } from "./fleet-events";

const entry = (id: number, job: string, says: string): TimelineEvent => ({
  id,
  at: new Date().toISOString(),
  kind: "job.step",
  subject: { kind: "job", id: job },
  title: `Job ${job}`,
  tone: "run",
  says,
  live: true,
});

const HELD: readonly TimelineEvent[] = [entry(1, "a", "Scope passed"), entry(2, "b", "Implement started")];

let off: (() => void) | undefined;
afterEach(() => off?.());

function feed(props: Partial<Parameters<typeof ActivityFeed>[0]> = {}) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  off = () => {
    root.unmount();
    host.remove();
  };
  root.render(
    <FleetEventsOver source={{ get: () => HELD, subscribe: () => () => undefined }}>
      <ActivityFeed now={Date.now()} jobs={[]} filter="running" onOpenJob={() => {}} onOpenSession={() => {}} {...props} />
    </FleetEventsOver>,
  );
}

const row = (id: string) => document.querySelector<HTMLElement>(`[data-event="${id}"]`);
const rows = () => [...document.querySelectorAll("[data-event]")].map((one) => one.getAttribute("data-event")).sort();

test("keep narrows the feed to the subjects it keeps, and absent keeps every one", async () => {
  feed();
  await expect.poll(rows).toEqual(["1", "2"]);
  off?.();

  feed({ keep: (subject) => subject.kind === "job" && subject.id === "a" });
  await expect.poll(rows).toEqual(["1"]);
});

test("onPick fires with the event whose row was pressed, and the subject does not open", async () => {
  const picked = vi.fn<(event: FleetEvent) => void>();
  const opened = vi.fn();
  feed({ onPick: picked, onOpenJob: opened });
  await expect.poll(rows).toEqual(["1", "2"]);

  await userEvent.click(row("2")!.querySelector(".armada-tl__says")!);

  expect(picked).toHaveBeenCalledTimes(1);
  expect(picked.mock.calls[0]![0]).toMatchObject({ id: "2", subject: { kind: "job", id: "b" } });
  // Pressing the name is a pick too: the deck decides what a pick opens.
  await page.getByRole("button", { name: "Job: Job a" }).click();
  expect(picked.mock.calls[1]![0]).toMatchObject({ id: "1" });
  expect(opened).not.toHaveBeenCalled();
});

test("without onPick a press on the name opens its subject, as the Timeline's does", async () => {
  const opened = vi.fn();
  feed({ onOpenJob: opened });

  await page.getByRole("button", { name: "Job: Job b" }).click();

  expect(opened).toHaveBeenCalledWith("b");
});

test("lit holds one row, and stacked draws the deck's rows", async () => {
  feed({ lit: "2", stacked: true });
  await expect.poll(rows).toEqual(["1", "2"]);

  expect(row("2")!.hasAttribute("data-lit")).toBe(true);
  expect(row("1")!.hasAttribute("data-lit")).toBe(false);
  expect(row("2")!.hasAttribute("data-stacked")).toBe(true);
});
